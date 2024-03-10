use std::collections::HashMap;

use clap::Parser;
use color_eyre::eyre::{eyre, OptionExt};
use color_eyre::owo_colors::OwoColorize;
use color_eyre::{eyre, eyre::bail};
use futures::TryStreamExt;

use chris::errors::CubeError;
use chris::types::{
    ComputeResourceName, CubeUrl, FeedId, PluginInstanceId, PluginParameterValue, Username,
};
use chris::{
    BaseChrisClient, ChrisClient, EitherClient, PipelineRw, PluginInstanceResponse,
    PluginInstanceRw, PluginRw,
};

use crate::arg::{GivenFeedOrPluginInstance, GivenPluginInstance, GivenRunnable, Runnable};
use crate::client::Credentials;
use crate::login::state::ChrsSessions;
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

    /// Parameters
    parameters: Vec<String>,
}

pub async fn run_command(credentials: Credentials, args: RunArgs) -> eyre::Result<()> {
    let (client, old, ui) = credentials
        .get_client([args.plugin_or_pipeline.as_arg_str()])
        .await?;
    if let EitherClient::LoggedIn(c) = client {
        run(c, old, ui, args).await
    } else {
        bail!("You are not logged in.")
    }
}

async fn run(
    client: ChrisClient,
    old: Option<PluginInstanceId>,
    ui: Option<UiUrl>,
    args: RunArgs,
) -> eyre::Result<()> {
    let runnable = args
        .plugin_or_pipeline
        .clone()
        .resolve_using(&client)
        .await?;
    match runnable {
        Runnable::Plugin(p) => run_plugin(client, p, old, ui, args).await,
        Runnable::Pipeline(p) => run_pipeline(client, p, ui, args).await,
    }
}

async fn run_plugin(
    client: ChrisClient,
    plugin: PluginRw,
    old: Option<PluginInstanceId>,
    ui: Option<UiUrl>,
    args: RunArgs,
) -> eyre::Result<()> {
    let (params, incoming) = clap_serialize_params(&plugin, &args.parameters).await?;
    let previous = get_input(&client, old, incoming).await?;
    let previous_id = previous.as_ref().map(|previous| previous.object.id.0);
    if !args.force {
        check_title(&client, previous.as_ref(), args.title.as_deref()).await?;
    }
    if args.dry_run {
        println!("Input: plugininstance/{:?}", previous_id);
        Ok(())
    } else {
        let created = create_plugin_instance(&plugin, params, previous_id, args).await?;
        if let Some(ui) = ui {
            let feed = created.feed().get().await?;
            println!("{}", ui.feed_url_of(&feed.object));
        }
        set_cd(client.url(), client.username(), created.object.id)
    }
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

/// Raise error if:
///
/// - title is None
/// - if has previous, title is not unique in the feed
/// - if no previous, title is not a unique feed name
async fn check_title(
    client: &ChrisClient,
    previous: Option<&PluginInstanceRw>,
    title: Option<&str>,
) -> eyre::Result<()> {
    if let Some(title) = title {
        if let Some(plinst) = previous {
            if title_is_not_unique(client, &plinst.object, title).await? {
                bail!(
                    "Title is not unique within the feed. {}",
                    "You can bypass this check using --force".dimmed()
                );
            }
        } else if feed_name_is_not_unique(client, title).await? {
            bail!(
                "Title is not a unique feed name. {}",
                "You can bypass this check using --force".dimmed()
            );
        }
    } else {
        bail!(
            "Please provide a value for {}. {}",
            "--title".bold(),
            "You can bypass this check using --force".dimmed()
        )
    };
    Ok(())
}

fn set_cd(cube_url: &CubeUrl, username: &Username, id: PluginInstanceId) -> eyre::Result<()> {
    let mut sessions = ChrsSessions::load()?;
    if sessions.set_plugin_instance(cube_url, username, id) {
        sessions.save()?;
    }
    Ok(())
}

async fn title_is_not_unique(
    client: &ChrisClient,
    plinst: &PluginInstanceResponse,
    title: &str,
) -> Result<bool, CubeError> {
    let feed_id = plinst.feed_id;
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

async fn get_input(
    client: &ChrisClient,
    old: Option<PluginInstanceId>,
    given: Option<GivenFeedOrPluginInstance>,
) -> eyre::Result<Option<PluginInstanceRw>> {
    if let Some(feed_or_plinst) = given {
        get_feed_or_plinst(client, old, feed_or_plinst)
            .await
            .map(Some)
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

async fn get_feed_or_plinst(
    client: &ChrisClient,
    old: Option<PluginInstanceId>,
    feed_or_plinst: GivenFeedOrPluginInstance,
) -> eyre::Result<PluginInstanceRw> {
    match feed_or_plinst {
        GivenFeedOrPluginInstance::FeedId(id) => get_plinst_of_feed(client, id).await,
        GivenFeedOrPluginInstance::FeedName(name) => {
            let feed_id = get_feedrw_by_name(client, name).await?;
            get_plinst_of_feed(client, feed_id).await
        }
        GivenFeedOrPluginInstance::PluginInstance(given) => given.get_using_rw(client, old).await,
        GivenFeedOrPluginInstance::Ambiguous(value) => {
            GivenPluginInstance::from(value)
                .get_using_rw(client, old)
                .await
        }
    }
}

/// Get the first plugin instance of a feed returned from CUBE's API,
/// which we assume to be the most recently created plugin instance
/// of that feed.
async fn get_plinst_of_feed(
    client: &ChrisClient,
    feed_id: FeedId,
) -> eyre::Result<PluginInstanceRw> {
    let query = client
        .plugin_instances()
        .feed_id(feed_id)
        .page_limit(1)
        .max_items(1);
    let search = query.search();
    search.get_first().await?.ok_or_else(|| {
        eyre!(
            "feed/{} does not contain plugin instances. This is a CUBE bug.",
            feed_id.0
        )
    })
}

async fn get_feedrw_by_name(client: &ChrisClient, name: String) -> color_eyre::Result<FeedId> {
    let query = client.feeds().name_exact(name).page_limit(2).max_items(2);
    let search = query.search();
    let items: Vec<_> = search.stream().map_ok(|f| f.id).try_collect().await?;
    if items.len() > 1 {
        bail!("Multiple feeds found, please be more specific.\nHint: run `{}` and specify feed by feed/{}", "chrs list".bold(), "ID".bold().green())
    }
    items.into_iter().next().ok_or_eyre("Feed not found")
}

async fn run_pipeline(
    client: ChrisClient,
    plugin: PipelineRw,
    ui: Option<UiUrl>,
    args: RunArgs,
) -> eyre::Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use fake::Fake;
    use rstest::*;

    use chris::Account;

    use super::*;

    #[fixture]
    fn cube_url() -> CubeUrl {
        CubeUrl::from_static("https://cube-for-testing-chrisui.apps.shift.nerc.mghpcc.org/api/v1/")
    }

    #[fixture]
    #[once]
    fn credentials(cube_url: CubeUrl) -> Credentials {
        let username: String = fake::faker::internet::en::Username().fake();
        let email: String = fake::faker::internet::en::SafeEmail().fake();
        let password = format!("{}1234", &username.chars().rev().collect::<String>());
        let username = Username::new(username);
        let token = futures::executor::block_on(async {
            let account_creator = Account::new(&cube_url, &username, &password);
            account_creator.create_account(&email).await.unwrap();
            account_creator.get_token().await.unwrap()
        });
        Credentials {
            cube_url: Some(cube_url),
            username: Some(username),
            password: None,
            token: Some(token),
            retries: None,
            ui: None,
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
            assert!(error.to_string().contains("Please provide a value for"))
        } else {
            panic!("Expected an error to happen because no title was given.")
        }
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_everything(credentials: &Credentials) {
        let c = credentials.clone();
        let client = ChrisClient::build(c.cube_url.unwrap(), c.username.unwrap(), c.token.unwrap())
            .unwrap()
            .connect()
            .await
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
            .search()
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
                .contains("Title is not unique within the feed."));
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
    }

    fn uuid_name(name: &str) -> String {
        format!(
            "chrs test -- {} -- {}",
            name,
            uuid::Uuid::new_v4().hyphenated().to_string()
        )
    }

    fn create_args(title: Option<String>, plugin: &str, args: &[&str]) -> RunArgs {
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
            plugin_or_pipeline: GivenRunnable::try_from(plugin.to_string()).unwrap(),
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
            parameters: args.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}
