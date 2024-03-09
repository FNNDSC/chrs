use clap::Parser;
use color_eyre::eyre;
use color_eyre::owo_colors::OwoColorize;
use dialoguer::console::Term;
use futures::TryStreamExt;
use time::format_description::well_known::Rfc2822;

use chris::errors::CubeError;
use chris::{
    Access, EitherClient, Pipeline, PipelineRw, Plugin, PluginParameter, PluginResponse, PluginRw,
};

use crate::arg::{GivenRunnable, Runnable};
use crate::client::Credentials;
use crate::login::{UiUrl, UiUrlRef};
use crate::plugin_clap::clap_params;

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
        EitherClient::Anon(c) => match args.plugin_or_pipeline.resolve_using(c).await? {
            Runnable::Plugin(p) => describe_plugin_ro(&p, ui).await,
            Runnable::Pipeline(p) => describe_pipeline_ro(&p, ui).await,
        },
        EitherClient::LoggedIn(c) => match args.plugin_or_pipeline.resolve_using(c).await? {
            Runnable::Plugin(p) => describe_plugin_rw(&p, ui).await,
            Runnable::Pipeline(p) => {
                describe_pipeline_ro(&p, ui).await?;
                println!();
                print_pipeline_workflow_counts(&p).await
            }
        },
    }
}

fn print_plugin_title(plugin: &PluginResponse, ui: Option<&UiUrlRef>) {
    let id_part = format!("(plugin/{})", plugin.id.0);
    println!(
        "{}: {} {}",
        plugin.name.bold().underline().magenta(),
        plugin.title,
        id_part.dimmed(),
    );
    if let Some(ui) = ui {
        println!("{}/plugin/{}", ui, plugin.id.0)
    }
}

fn get_plugin_attributes<'a>(plugin: &'a PluginResponse) -> Vec<(&'static str, &'a str)> {
    let mut attributes = vec![
        ("Version", plugin.version.as_str()),
        ("Image", plugin.dock_image.as_str()),
        ("License", plugin.license.as_str()),
        ("Code", plugin.public_repo.as_str()),
    ];
    if plugin.documentation != plugin.public_repo.as_str() {
        attributes.push(("Documentation", plugin.documentation.as_str()));
    }
    attributes
}

async fn describe_plugin_ro<A: Access>(plugin: &Plugin<A>, ui: Option<UiUrl>) -> eyre::Result<()> {
    print_plugin_title(&plugin.object, ui.as_deref());
    println!();
    for (name, val) in get_plugin_attributes(&plugin.object) {
        println!("{:>16}: {}", name, val)
    }
    println!();
    let params = get_parameters(plugin).await?;
    clap_params(&plugin.object.selfexec, &params).print_help()?;
    Ok(())
}

async fn describe_plugin_rw(plugin: &PluginRw, ui: Option<UiUrl>) -> eyre::Result<()> {
    print_plugin_title(&plugin.object, ui.as_deref());
    println!();
    let mut attributes = get_plugin_attributes(&plugin.object);
    let cr_names = compute_resources_of(plugin).await?;
    attributes.push(("Compute Resources", &cr_names));
    for (name, val) in attributes {
        println!("{:>20}: {}", name, val)
    }
    println!();
    let params = get_parameters(plugin).await?;
    clap_params(&plugin.object.selfexec, &params).print_help()?;
    Ok(())
}

async fn get_parameters<A: Access>(plugin: &Plugin<A>) -> Result<Vec<PluginParameter>, CubeError> {
    let parameters = plugin.parameters();
    let parameter_search = parameters.search();
    parameter_search.stream().try_collect().await
}

async fn compute_resources_of(plugin: &PluginRw) -> Result<String, CubeError> {
    let cr = plugin.compute_resources();
    let cr_search = cr.search();
    cr_search
        .stream()
        .map_ok(|c| c.name)
        .try_collect::<Vec<_>>()
        .await
        .map(|v| v.join(", "))
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

async fn print_pipeline_workflow_counts(pipeline: &PipelineRw) -> eyre::Result<()> {
    let workflows_search = pipeline.get_workflows();
    let count = workflows_search.search().get_count().await?;
    if count == 1 {
        println!("Pipeline was used {} time", 1.bold().bright_cyan());
    } else {
        println!("Pipeline was used {} times", count.bold().bright_cyan());
    }
    Ok(())
}
