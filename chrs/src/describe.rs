use clap::Parser;
use color_eyre::eyre;
use color_eyre::owo_colors::OwoColorize;
use dialoguer::console::Term;
use time::format_description::well_known::Rfc2822;

use chris::{Access, Pipeline, PipelineRw, Plugin};

use crate::arg::{GivenRunnable, Runnable};
use crate::client::{Client, Credentials};
use crate::login::UiUrl;

#[derive(Parser)]
pub struct DescribeArgs {
    /// Plugin or pipeline
    plugin_or_pipeline: GivenRunnable,
}

pub async fn describe_runnable(credentials: Credentials, args: DescribeArgs) -> eyre::Result<()> {
    let (client, _, ui) = credentials
        .get_client([args.plugin_or_pipeline.as_arg_str()])
        .await?;
    match &client {
        Client::Anon(c) => match args.plugin_or_pipeline.resolve_using(c).await? {
            Runnable::Plugin(p) => describe_plugin_ro(&p, ui).await,
            Runnable::Pipeline(p) => describe_pipeline_ro(&p, ui).await,
        },
        Client::LoggedIn(c) => match args.plugin_or_pipeline.resolve_using(c).await? {
            Runnable::Plugin(p) => describe_plugin_ro(&p, ui).await,
            Runnable::Pipeline(p) => {
                describe_pipeline_ro(&p, ui).await?;
                println!();
                describe_pipeline_authed(&p).await
            }
        },
    }
}

async fn describe_plugin_ro<A: Access>(
    plugin: &Plugin<A>,
    ui: Option<UiUrl>,
) -> eyre::Result<()> {
    println!("Got a plugin");
    Ok(())
}

async fn describe_pipeline_ro<A: Access>(
    pipeline: &Pipeline<A>,
    _ui: Option<UiUrl>,
) -> eyre::Result<()> {
    let id_part = format!("(pipeline/{})", pipeline.object.id.0);
    println!(
        "{} {}",
        pipeline.object.name.bold().underline().bright_magenta(),
        id_part.dimmed()
    );
    println!("  Category: {}", pipeline.object.category.bold());
    println!("   Authors: {}", pipeline.object.authors);
    println!(
        "   Created: {}",
        pipeline.object.creation_date.format(&Rfc2822)?
    );
    println!();
    let term_cols = std::cmp::min(Term::stdout().size().1, 120) as usize;
    for line in textwrap::wrap(pipeline.object.description.as_str(), term_cols) {
        println!("{}", line)
    }
    Ok(())
}

async fn describe_pipeline_authed(
    pipeline: &PipelineRw,
) -> eyre::Result<()> {
    let workflows_search = pipeline.get_workflows();
    let count = workflows_search.search().get_count().await?;
    if count == 1 {
        println!("Pipeline was used {} time", 1.bold().bright_cyan());
    } else {
        println!("Pipeline was used {} times", count.bold().bright_cyan());
    }
    Ok(())
}
