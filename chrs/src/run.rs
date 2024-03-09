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
    let (mut params, incoming) = clap_serialize_params(&plugin, &args.parameters).await?;
    let previous = get_input(&client, old, incoming).await?;
    let previous_id = previous.object.id.0;
    if !args.force {
        if let Some(title) = &args.title {
            if title_is_not_unique(&client, &previous.object, title).await? {
                bail!(
                    "Title is not unique within the feed. {}",
                    "You can bypass this check using --force".dimmed()
                );
            }
        } else {
            bail!(
                "Please provide a value for {}. {}",
                "--title".bold(),
                "You can bypass this check using --force".dimmed()
            )
        }
    }
    if args.dry_run {
        println!("Input: plugininstance/{}", previous_id);
        Ok(())
    } else {
        if let Some(title) = args.title {
            params.insert("title".to_string(), PluginParameterValue::Stringish(title));
        }
        params.insert(
            "previous_id".to_string(),
            PluginParameterValue::Integer(previous_id as i64),
        );
        let created = plugin.create_instance(&params).await?;
        if let Some(ui) = ui {
            let feed = created.feed().get().await?;
            println!("{}", ui.feed_url_of(&feed.object));
        }
        set_cd(client.url(), client.username(), created.object.id)
    }
}

fn set_cd(cube_url: &CubeUrl, username: &Username, id: PluginInstanceId) -> eyre::Result<()> {
    let mut sessions = ChrsSessions::load()?;
    sessions.set_plugin_instance(cube_url, username, id);
    sessions.save()
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

async fn run_pipeline(
    client: ChrisClient,
    plugin: PipelineRw,
    ui: Option<UiUrl>,
    args: RunArgs,
) -> eyre::Result<()> {
    todo!()
}

async fn get_input(
    client: &ChrisClient,
    old: Option<PluginInstanceId>,
    given: Option<GivenFeedOrPluginInstance>,
) -> eyre::Result<PluginInstanceRw> {
    if let Some(feed_or_plinst) = given {
        get_feed_or_plinst(client, old, feed_or_plinst).await
    } else if let Some(id) = old {
        client
            .get_plugin_instance(id)
            .await
            .map_err(eyre::Error::new)
    } else {
        bail!("Input plugin instance or feed not specified.")
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
