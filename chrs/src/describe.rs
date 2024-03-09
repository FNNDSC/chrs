use clap::Parser;
use color_eyre::eyre;
use chris::{LinkedModel, PipelineResponse, PublicPlugin, RoAccess};

use crate::arg::{GivenRunnable, Runnable};
use crate::client::{Client, Credentials};

#[derive(Parser)]
pub struct DescribeArgs {
    /// Plugin or pipeline
    plugin_or_pipeline: GivenRunnable,
}

pub async fn describe_runnable(credentials: Credentials, args: DescribeArgs) -> eyre::Result<()> {
    let (client, _, ui) = credentials
        .get_client([args.plugin_or_pipeline.as_arg_str()])
        .await?;
    let runnable = match &client {
        Client::Anon(c) => args.plugin_or_pipeline.resolve_using(c).await?,
        Client::LoggedIn(c) => args.plugin_or_pipeline.resolve_using(c).await?.into_ro(),
    };
    match runnable {
        Runnable::Plugin(p) => describe_plugin(p).await,
        Runnable::Pipeline(p) => describe_pipeline(p).await,
    }
}

async fn describe_plugin(plugin: PublicPlugin) -> eyre::Result<()> {
    println!("Got a plugin");
    Ok(())
}

async fn describe_pipeline(pipeline: LinkedModel<PipelineResponse, RoAccess>) -> eyre::Result<()> {
    println!("Got a pipeline");
    Ok(())
}
