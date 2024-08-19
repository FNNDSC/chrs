use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;

use clap::Parser;
use color_eyre::eyre::{eyre, OptionExt, WrapErr};
use color_eyre::owo_colors::OwoColorize;
use color_eyre::{eyre, eyre::bail};
use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use tokio::try_join;

use chris::errors::CubeError;
use chris::types::{ComputeResourceName, PluginInstanceId, PluginParameterValue};
use chris::{
    BaseChrisClient, ChrisClient, EitherClient, PipelineRw, PluginInstanceResponse,
    PluginInstanceRw, PluginRw,
};

use crate::arg::{GivenDataNode, GivenRunnable, Runnable};
use crate::credentials::Credentials;
use crate::login::UiUrl;
use crate::plugin_clap::clap_serialize_params;

#[derive(Parser)]
pub struct RunArgs {
    /// CPU resource request, as number of CPU cores.
    #[clap(short = 'J', long, value_name = "N")]
    cpu: Option<u32>,

    /// CPU resource request.
    /// Format is xm where x is an integer in millicores.
    #[clap(long, conflicts_with = "cpu")]
    cpu_limit: Option<String>,

    /// Memory resource request.
    /// Format is xMi or xGi where x is an integer.
    #[clap(short, long)]
    memory_limit: Option<String>,

    /// GPU resource request.
    /// Number of GPUs to use for plugin instance.
    #[clap(short, long)]
    gpu_limit: Option<u32>,

    /// Number of workers resource request.
    /// Number of compute nodes for parallel job.
    #[clap(short, long)]
    number_of_workers: Option<u32>,

    /// Name of compute resource
    #[clap(short, long)]
    compute_resource_name: Option<ComputeResourceName>,

    /// Plugin instance title
    #[clap(short, long)]
    title: Option<String>,

    /// Bypass checks of best practices
    #[clap(short, long)]
    force: bool,

    /// Do not actually run
    #[clap(short, long)]
    dry_run: bool,

    /// Plugin or pipeline to run
    #[clap(required = true)]
    plugin_or_pipeline: GivenRunnable,

    /// Maximum number of concurrent HTTP requests
    #[clap(short = 'j', long, default_value_t = 4)]
    threads: usize,

    /// Plugin parameters and/or plugin/pipeline inputs
    parameters: Vec<String>,
}

pub async fn run_command(credentials: Credentials, args: RunArgs) -> eyre::Result<()> {
    let (client, old, ui) = credentials
        .clone()
        .get_client([args.plugin_or_pipeline.as_arg_str()])
        .await?;
    let client = if let EitherClient::LoggedIn(logged_in_client) = client {
        Ok(logged_in_client)
    } else {
        Err(eyre!(
            "This command is only available for authenticated users. Try running `{}` with a username first.",
            "chrs login".bold()
        ))
    }?;
    if let Some(id) = run(&client, old, ui, args).await? {
        crate::login::set_cd(client.url(), client.username(), id, credentials.config_path)?;
        println!("plugininstance/{}", id.0);
    }
    Ok(())
}

async fn run(
    client: &ChrisClient,
    old: Option<PluginInstanceId>,
    ui: Option<UiUrl>,
    args: RunArgs,
) -> eyre::Result<Option<PluginInstanceId>> {
    let (title_is_unique, runnable) = try_join!(
        check_title(client, old, args.title.as_deref(), args.force),
        args.plugin_or_pipeline.clone().resolve_using(client)
    )?;
    if let Some(error) = title_is_unique {
        bail!("{}", error);
    }
    let plinst = match runnable {
        Runnable::Plugin(p) => run_plugin(client, p, old, args).await,
        Runnable::Pipeline(p) => run_pipeline(client, p, old, args).await,
    }?;
    if let (Some(ui), Some(plinst)) = (ui, plinst.as_ref()) {
        let feed = plinst.feed().get().await?;
        let feed_ui_url = ui.feed_url_of(&feed.object);
        eprintln!("{}", feed_ui_url);
    }
    Ok(plinst.map(|p| p.object.id))
}

async fn run_plugin(
    client: &ChrisClient,
    plugin: PluginRw,
    old: Option<PluginInstanceId>,
    args: RunArgs,
) -> eyre::Result<Option<PluginInstanceRw>> {
    let (params, incoming) = clap_serialize_params(&plugin, &args.parameters).await?;
    let previous = get_input(client, old, incoming, args.threads).await?;
    let previous_id = previous.as_ref().map(|previous| previous.object.id.0);
    if args.dry_run {
        eprintln!("Input: plugininstance/{:?}", previous_id);
        Ok(None)
    } else {
        create_plugin_instance(&plugin, params, previous_id, args)
            .await
            .map(Some)
    }
}

async fn run_pipeline(
    client: &ChrisClient,
    pipeline: PipelineRw,
    old: Option<PluginInstanceId>,
    args: RunArgs,
) -> eyre::Result<Option<PluginInstanceRw>> {
    let inputs: Vec<GivenDataNode> = args.parameters.into_iter().map(|p| p.into()).collect();
    let prev = get_input(client, old, inputs, args.threads)
        .await?
        .ok_or_eyre("Missing operand")?;
    let workflow = pipeline
        .create_workflow(prev.object.id, args.title.as_deref())
        .await?;
    // get the "last" plugin instance created by the workflow. Assumes CUBE returns the plugin instances in order.
    workflow
        .plugin_instances()
        .get_first()
        .await
        .map_err(eyre::Error::new)
}

/// Create a plugin instance. If the plugin is a fs-type plugin, then the created feed name
/// is set to the plugin instance's title.
async fn create_plugin_instance(
    plugin: &PluginRw,
    mut params: HashMap<String, PluginParameterValue>,
    previous_id: Option<u32>,
    args: RunArgs,
) -> eyre::Result<PluginInstanceRw> {
    let title = args.title.clone();
    let optional_resources = serialize_optional_resources(args, previous_id);
    params.extend(optional_resources);
    let created = plugin.create_instance(&params).await?;
    if previous_id.is_none() {
        if let Some(title) = title {
            let feed = created.feed();
            feed.set_name(&title).await?;
        }
    }
    Ok(created)
}

fn serialize_optional_resources(
    args: RunArgs,
    previous_id: Option<u32>,
) -> impl Iterator<Item = (String, PluginParameterValue)> {
    let cpu_limit = args
        .cpu
        .map(|c| format!("{}m", c * 1000))
        .or(args.cpu_limit);
    let optional_resources = [
        cpu_limit.map(|v| ("cpu_limit".to_string(), PluginParameterValue::Stringish(v))),
        args.memory_limit.map(|v| {
            (
                "memory_limit".to_string(),
                PluginParameterValue::Stringish(v),
            )
        }),
        args.gpu_limit.map(|v| {
            (
                "gpu_limit".to_string(),
                PluginParameterValue::Integer(v as i64),
            )
        }),
        args.number_of_workers.map(|v| {
            (
                "number_of_workers".to_string(),
                PluginParameterValue::Integer(v as i64),
            )
        }),
        args.compute_resource_name.map(|v| {
            (
                "compute_resource_name".to_string(),
                PluginParameterValue::Stringish(v.to_string()),
            )
        }),
        args.title
            .map(|v| ("title".to_string(), PluginParameterValue::Stringish(v))),
        previous_id.map(|v| {
            (
                "previous_id".to_string(),
                PluginParameterValue::Integer(v as i64),
            )
        }),
    ];
    optional_resources.into_iter().flatten()
}

async fn check_title(
    client: &ChrisClient,
    old: Option<PluginInstanceId>,
    title: Option<&str>,
    force: bool,
) -> eyre::Result<Option<TitleUniqueness>> {
    if force {
        return Ok(None);
    }
    if let Some(title) = title {
        if let Some(id) = old {
            if title_is_not_unique(client, id, title).await? {
                return Ok(Some(TitleUniqueness::NotUniqueWithinFeed));
            }
        } else if feed_name_is_not_unique(client, title).await? {
            return Ok(Some(TitleUniqueness::NotUniqueFeedName));
        }
    } else {
        return Ok(Some(TitleUniqueness::NoTitle));
    };
    Ok(None)
}

enum TitleUniqueness {
    NotUniqueWithinFeed,
    NotUniqueFeedName,
    NoTitle,
}

impl Display for TitleUniqueness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hint = "You can bypass this check using --force";
        let msg = match self {
            TitleUniqueness::NotUniqueWithinFeed => {
                Cow::Borrowed("Title is not unique within feed.")
            }
            TitleUniqueness::NotUniqueFeedName => Cow::Borrowed("Title is not a unique feed name."),
            TitleUniqueness::NoTitle => Cow::Owned(format!("A {} is required.", "--title".bold())),
        };
        write!(f, "{} {}", msg, hint.dimmed())
    }
}

async fn title_is_not_unique(
    client: &ChrisClient,
    plinst: PluginInstanceId,
    title: &str,
) -> Result<bool, CubeError> {
    let feed_id = client.get_plugin_instance(plinst).await?.object.feed_id;
    let query = client
        .plugin_instances()
        .feed_id(feed_id)
        .title(title.to_string());
    let search = query.search();
    search.get_count().await.map(|count| count > 0)
}

async fn feed_name_is_not_unique(client: &ChrisClient, name: &str) -> Result<bool, CubeError> {
    let query = client.feeds().name_exact(name);
    let search = query.search();
    search.get_count().await.map(|count| count > 0)
}

/// Picks a plugin instance to use as the input.
///
/// - If `given` is of length one: get it as a plugin instance.
/// - If `given` has length > 1: run `pl-topologicalcopy` and return that
/// - If `given` has length = 0: get `old` and return that
async fn get_input(
    client: &ChrisClient,
    old: Option<PluginInstanceId>,
    given: Vec<GivenDataNode>,
    threads: usize,
) -> eyre::Result<Option<PluginInstanceRw>> {
    if given.len() > 1 {
        return topologicalcopy(client, old, given, threads).await.map(Some);
    }
    if let Some(feed_or_plinst) = given.into_iter().next() {
        feed_or_plinst.into_plinst_rw(client, old).await.map(Some)
    } else if let Some(id) = old {
        client
            .get_plugin_instance(id)
            .await
            .map(Some)
            .map_err(eyre::Error::new)
    } else {
        Ok(None)
    }
}

/// Run `pl-topologicalcopy`
async fn topologicalcopy(
    client: &ChrisClient,
    old: Option<PluginInstanceId>,
    given: Vec<GivenDataNode>,
    threads: usize,
) -> eyre::Result<PluginInstanceRw> {
    let previous: Vec<_> = futures::stream::iter(given)
        .map(|p| async move { p.into_plinst_rw(client, old).await.map(|p| p.object) })
        .map(Ok::<_, eyre::Error>)
        .try_buffered(threads)
        .try_collect()
        .await?;
    let topologicalcopy = client
        .plugin()
        .name_exact("pl-topologicalcopy")
        .version("1.0.2")
        .search()
        .get_only()
        .await
        .wrap_err("pl-topologicalcopy@1.0.2 not found")?;
    let params = TopologicalCopyParameters::new(&previous);
    let created = topologicalcopy.create_instance(&params).await?;
    Ok(created)
}

#[derive(serde::Serialize)]
struct TopologicalCopyParameters {
    previous_id: PluginInstanceId,
    plugininstances: String,
    title: String,
}

impl TopologicalCopyParameters {
    fn new(previous: &[PluginInstanceResponse]) -> Self {
        let title = format!(
            "Merge of: {}",
            previous
                .iter()
                .map(quoted_title_of_plinst_response)
                .join(" ")
        );
        // CUBE does not allow plugin instance titles to be longer than 100 characters.
        let title = if title.len() > 100 {
            format!("{}...", &title[..97])
        } else {
            title
        };
        Self {
            previous_id: previous.first().unwrap().id,
            plugininstances: previous.iter().map(|p| p.id.0.to_string()).join(","),
            title,
        }
    }
}

fn quoted_title_of_plinst_response(p: &PluginInstanceResponse) -> String {
    if p.title.is_empty() {
        format!("{}#{}", p.plugin_name, p.id.0)
    } else {
        format!("\"{}\"", p.title)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use fake::Fake;
    use futures::TryStreamExt;
    use rstest::*;
    use tempfile::TempDir;

    use chris::types::{CubeUrl, Username};
    use chris::Account;

    use crate::credentials::NO_ARGS;
    use crate::login::state::ChrsSessions;
    use crate::login::store::{SavedCubeState, StoredToken};

    use super::*;

    #[fixture]
    fn cube_url() -> CubeUrl {
        CubeUrl::from_static("https://cube-for-testing-chrisui.apps.shift.nerc.mghpcc.org/api/v1/")
    }

    #[fixture]
    #[once]
    fn tmp_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[fixture]
    #[once]
    fn config_path(tmp_dir: &TempDir) -> Option<PathBuf> {
        let u = uuid::Uuid::new_v4().hyphenated().to_string();
        Some(tmp_dir.path().join(format!("{u}.ron")))
    }

    #[fixture]
    #[once]
    fn credentials(cube_url: CubeUrl, config_path: &Option<PathBuf>) -> Credentials {
        let username: String = fake::faker::internet::en::Username().fake();
        let email: String = fake::faker::internet::en::SafeEmail().fake();
        let password = format!("{}1234", &username.chars().rev().collect::<String>());
        let username = Username::new(username);
        let token = futures::executor::block_on(async {
            let account_creator = Account::new(&cube_url, &username, &password);
            account_creator.create_account(&email).await.unwrap();
            account_creator.get_token().await.unwrap()
        });
        let sessions = ChrsSessions {
            sessions: vec![SavedCubeState {
                cube: cube_url.clone(),
                username: username.clone(),
                store: StoredToken::Text(token),
                current_plugin_instance_id: None,
                ui: None,
            }],
        };
        // save token to storage
        sessions.save(config_path.as_deref()).unwrap();
        Credentials {
            cube_url: Some(cube_url),
            username: Some(username),
            password: None,
            token: None, // token will be looked up from storage
            retries: None,
            ui: None,
            config_path: config_path.clone(),
        }
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_gives_warning_for_no_title(credentials: &Credentials) {
        if let Err(error) = run_command(
            credentials.clone(),
            create_args(None, "pl-mri10yr06mo01da_normal", &[]),
        )
        .await
        {
            assert!(error
                .to_string()
                .contains(TitleUniqueness::NoTitle.to_string().as_str()))
        } else {
            panic!("Expected an error to happen because no title was given.")
        }
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_everything(credentials: &Credentials) {
        let client = credentials
            .clone()
            .get_client(NO_ARGS)
            .await
            .unwrap()
            .0
            .logged_in()
            .unwrap();

        let title = uuid_name("first title");
        run_command(
            credentials.clone(),
            create_args(Some(title.clone()), "pl-mri10yr06mo01da_normal@1.1.4", &[]),
        )
        .await
        .unwrap();
        let first_plinst = client
            .plugin_instances()
            .title(&title)
            .search()
            .get_only()
            .await
            .expect("Expected plugin instance to have been created with given title.");
        let feed = client.feeds().name_exact(&title).search().get_only().await.expect("Expected feed to be created with same name as given title, since plugin is a FS-type plugin.");
        assert_eq!(first_plinst.object.feed_id, feed.object.id);

        let feed_name = uuid_name("renamed feed");
        let _feed = feed.set_name(&feed_name).await.unwrap();
        let feed_by_name = format!("feed/{}", &feed_name);
        let first_plinst_by_name = format!("pi/{}", &title);
        let second_title = uuid_name("second title");
        run_command(
            credentials.clone(),
            create_args(
                Some(second_title.clone()),
                "pl-dcm2niix@0.1.0",
                &["-b", "n", &first_plinst_by_name],
            ),
        )
        .await
        .unwrap();
        let second_plinst = client
            .plugin_instances()
            .title(&second_title)
            .search()
            .get_only()
            .await
            .unwrap();
        assert_eq!(
            second_plinst.object.previous_id.unwrap(),
            first_plinst.object.id
        );
        let actual: HashMap<_, _> = second_plinst
            .parameters()
            .stream()
            .map_ok(|p| (p.param_name, p.value))
            .try_collect()
            .await
            .unwrap();
        let expected: HashMap<_, _> = [(
            "b".to_string(),
            PluginParameterValue::Stringish("n".to_string()),
        )]
        .into_iter()
        .collect();
        assert_eq!(actual, expected, "Command-line parameters are not correct.");

        let third_run_fail = run_command(
            credentials.clone(),
            create_args(
                Some(second_title.clone()),
                "pl-mri-preview@1.2.0",
                &[&feed_by_name],
            ),
        )
        .await;
        if let Err(error) = third_run_fail {
            assert!(error
                .to_string()
                .contains(TitleUniqueness::NotUniqueWithinFeed.to_string().as_str()));
        } else {
            panic!("Expected an error message about non-unique plugin instance title.");
        }
        let third_title = uuid_name("third title");
        run_command(
            credentials.clone(),
            create_args_mem(
                Some(third_title.clone()),
                "pl-mri-preview@1.2.0",
                &[&feed_by_name],
                Some("1234Mi".to_string()),
            ),
        )
        .await
        .unwrap();
        let third_plinst = client
            .plugin_instances()
            .title(&third_title)
            .search()
            .get_only()
            .await
            .unwrap();
        assert_eq!(third_plinst.object.previous_id.unwrap(), second_plinst.object.id, "Specifying a feed should create the plugin instance after the most recent plugin instance of the feed.");
        assert_eq!(
            third_plinst.object.memory_limit, 1234,
            "Memory limit was specified"
        );

        let fourth_title = uuid_name("fourth title");
        run_command(
            credentials.clone(),
            create_args(
                Some(fourth_title.clone()),
                "pl-simpledsapp@2.0.2",
                &["--dummyFloat", "35.6"],
            ),
        )
        .await
        .unwrap();
        let fourth_plinst = client
            .plugin_instances()
            .title(&fourth_title)
            .search()
            .get_only()
            .await
            .unwrap();
        assert_eq!(fourth_plinst.object.previous_id.unwrap(), third_plinst.object.id, "Running another plugin instance without specifying input should use last plugin instance as input");

        let fifth_title = uuid_name("fifth title");
        run_command(
            credentials.clone(),
            create_args(
                Some(fifth_title.clone()),
                "pl-simpledsapp@2.0.2",
                &["--dummyInt", "108", ".."],
            ),
        )
        .await
        .unwrap();
        let fifth_plinst = client
            .plugin_instances()
            .title(&fifth_title)
            .search()
            .get_only()
            .await
            .unwrap();
        assert_eq!(
            fifth_plinst.object.previous_id, fourth_plinst.object.previous_id,
            "Specifying previous as \"..\" should create sibling plugin instance"
        );

        let sixth_title = uuid_name("sixth title");
        run_command(
            credentials.clone(),
            create_args(
                Some(sixth_title.clone()),
                "pl-simpledsapp@2.0.2",
                &[
                    "--dummyInt",
                    "789",
                    &third_plinst.object.title,
                    &fourth_plinst.object.title,
                    &fifth_plinst.object.title,
                ],
            ),
        )
        .await
        .unwrap();
        let sixth_plinst = client
            .plugin_instances()
            .title(&sixth_title)
            .search()
            .get_only()
            .await
            .unwrap();
        let topologicalcopy = client.plugin_instances()
            .feed_id(sixth_plinst.object.feed_id)
            .plugin_name("pl-topologicalcopy")
            .search()
            .get_only()
            .await
            .expect("Should run pl-topologicalcopy because mutiple previous plugin instances were specified.");
        assert_eq!(
            topologicalcopy.object.previous_id,
            fifth_plinst.object.previous_id,
        );
        assert_eq!(
            sixth_plinst.object.previous_id.unwrap(),
            topologicalcopy.object.id
        );
        let topo_params: HashMap<_, _> = topologicalcopy
            .parameters()
            .stream()
            .map_ok(|p| (p.param_name, p.value))
            .try_collect()
            .await
            .unwrap();
        let joined_ids_csv = topo_params
            .get("plugininstances")
            .expect("pl-topologicalcopy must be run with the --plugininstances parameter")
            .to_string();
        let joined_ids: HashSet<_> = joined_ids_csv.split(',').map(|s| s.to_string()).collect();
        let expected: HashSet<_> = [
            third_plinst.object.id,
            fourth_plinst.object.id,
            fifth_plinst.object.id,
        ]
        .into_iter()
        .map(|id| id.0.to_string())
        .collect();
        assert_eq!(joined_ids, expected);

        let pipeline_name = "A pipeline to unstack directories and do nothing";
        let seventh_title = uuid_name("seventh title");
        run_command(
            credentials.clone(),
            create_args(Some(seventh_title.clone()), pipeline_name, &[]),
        )
        .await
        .unwrap();
        let workflow_instance = client
            .workflows()
            .pipeline_name(pipeline_name)
            .owner_username(client.username())
            .search()
            .get_only()
            .await
            .expect("Test user account should have created exactly one workflow.");
        let workflow_plinst_count = client
            .plugin_instances()
            .feed_id(sixth_plinst.object.feed_id)
            .workflow_id(workflow_instance.object.id)
            .search()
            .get_count()
            .await
            .unwrap();
        assert_eq!(
            workflow_plinst_count, 2,
            "Workflow should have created 2 plugin instances in the current feed."
        );
    }

    fn uuid_name(name: &str) -> String {
        format!(
            "chrs test -- {} -- {}",
            name,
            uuid::Uuid::new_v4().hyphenated().to_string()
        )
    }

    fn create_args(title: Option<String>, plugin_or_pipeline: &str, args: &[&str]) -> RunArgs {
        RunArgs {
            cpu: None,
            cpu_limit: None,
            memory_limit: None,
            gpu_limit: None,
            number_of_workers: None,
            compute_resource_name: None,
            title,
            force: false,
            dry_run: false,
            plugin_or_pipeline: GivenRunnable::try_from(plugin_or_pipeline.to_string()).unwrap(),
            threads: 4,
            parameters: args.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    fn create_args_mem(
        title: Option<String>,
        plugin: &str,
        args: &[&str],
        memory_limit: Option<String>,
    ) -> RunArgs {
        RunArgs {
            cpu: None,
            cpu_limit: None,
            memory_limit,
            gpu_limit: None,
            number_of_workers: None,
            compute_resource_name: None,
            title,
            force: false,
            dry_run: false,
            plugin_or_pipeline: GivenRunnable::try_from(plugin.to_string()).unwrap(),
            threads: 4,
            parameters: args.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}
