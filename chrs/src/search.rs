use crate::credentials::{Credentials, NO_ARGS};
use chris::errors::CubeError;
use chris::{PipelineResponse, PluginResponse};
use clap::Parser;
use color_eyre::eyre;
use color_eyre::eyre::Result;
use color_eyre::owo_colors::OwoColorize;
use futures::future::Ready;
use futures::{future, TryStreamExt};

#[derive(Parser)]
pub struct SearchArgs {
    /// Name to filter by
    #[clap(default_value = "")]
    name: String,
}

pub async fn search_runnable(credentials: Credentials, args: SearchArgs) -> Result<()> {
    let (client, _, _) = credentials.get_client(NO_ARGS).await?;
    let client_ro = client.into_ro();

    let plugin_query = client_ro.plugin().name_title_category(&args.name);
    let plugin_search = plugin_query.search();
    let plugins = plugin_search.stream().map_ok(format_plugin);

    let pipeline_query = client_ro.pipeline().name(&args.name);
    let pipeline_search = pipeline_query.search();
    let pipelines = pipeline_search.stream().map_ok(format_pipeline);

    let stream = tokio_stream::StreamExt::merge(plugins, pipelines);
    stream
        .try_for_each(print_string)
        .await
        .map_err(eyre::Error::new)
}

fn format_plugin(p: PluginResponse) -> String {
    let id = format!("{}/{}", "plugin".dimmed(), p.id.0);
    format!(
        "{:<22}{}{}{}",
        id.magenta(),
        p.name,
        "@".dimmed(),
        p.version.dimmed()
    )
}

fn format_pipeline(p: PipelineResponse) -> String {
    let id = format!("{}/{}", "pipeline".dimmed(), p.id.0);
    format!("{:<22}{}", id.bright_magenta(), p.name)
}

fn print_string(s: String) -> Ready<std::result::Result<(), CubeError>> {
    println!("{}", s);
    future::ok(())
}
