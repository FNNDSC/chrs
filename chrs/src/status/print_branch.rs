use std::fmt::Display;

use color_eyre::eyre::{bail, eyre, Result};
use color_eyre::eyre;
use color_eyre::owo_colors::OwoColorize;
use dialoguer::console::Term;
use futures::TryStreamExt;
use itertools::Itertools;
use tokio::try_join;

use chris::{FeedRo, PluginInstanceRo, PublicPlugin};
use chris::errors::CubeError;
use chris::types::SimplifiedStatus;

use crate::login::UiUrl;
use crate::unicode;

use super::feed::only_print_feed_status;
use super::find_branch::find_branch_to;

pub async fn print_branch_status(
    feed: FeedRo,
    selected: PluginInstanceRo,
    ui_url: Option<UiUrl>,
    threads: usize,
    show_execshell: bool
) -> Result<()> {
    only_print_feed_status(&feed, ui_url).await?;
    let all_plinst = get_all_plugin_instances(&feed).await?;
    let branch = find_branch_to(*selected.object.id, &all_plinst)
        .ok_or_else(|| eyre!("plugininstance/{} not found in feed, which contains plugin instances {}", *selected.object.id, all_plinst.iter().map(|p| p.object.id.0).join(",")))?;

    println!("\n{}", unicode::HORIZONTAL_BAR.repeat(40).dimmed());

    let term_cols = std::cmp::min(Term::stdout().size().1, 120) as usize;
    let branch_len = branch.len();
    for (i, plinst) in branch.into_iter().enumerate() {
        let is_current = plinst.object.id == selected.object.id;
        let has_next = i + 1 < branch_len;
        println!("{} {}", symbol_for(plinst), title_of(plinst, is_current));
        let pipe = if has_next { unicode::VERTICAL_BAR } else { " " };
        let cmd = cmd_of(plinst, threads, show_execshell).await?;
        let mut is_first = true;
        for line in textwrap::wrap(cmd.as_str(), term_cols) {
            let space = if is_first { " " } else { "     " };
            println!("{}{}{}", pipe.dimmed(), space, line.dimmed());
            is_first = false;
        }
        if has_next {
            println!("{}", pipe.dimmed())
        }
    }
    Ok(())
}

fn symbol_for(plinst: &PluginInstanceRo) -> impl Display {
    match plinst.object.status.simplify() {
        SimplifiedStatus::Waiting => unicode::DOTTED_CIRCLE.bold().to_string(),
        SimplifiedStatus::Running => unicode::BLACK_CIRCLE.bold().cyan().to_string(),
        SimplifiedStatus::Success => unicode::BLACK_CIRCLE.bold().blue().to_string(),
        SimplifiedStatus::Error => unicode::BLACK_CIRCLE.bold().bright_red().to_string(),
        SimplifiedStatus::Cancelled => unicode::WHITE_CIRCLE.dimmed().to_string()
    }
}

fn title_of(plinst: &PluginInstanceRo, is_current: bool) -> impl Display {
    let title = if plinst.object.title.is_empty() {
        plinst.object.plugin_name.as_str()
    } else {
        plinst.object.title.as_str()
    };
    if is_current {
        title.bold().to_string()
    } else {
        title.to_string()
    }
}

async fn cmd_of(plinst: &PluginInstanceRo, threads: usize, show_execshell: bool) -> Result<String, CubeError> {
    let plinst_parameters = plinst.parameters();
    let plinst_parameters_search = plinst_parameters.search();
    let (plugin, flags): (PublicPlugin, Vec<_>) = try_join!(
        plinst.plugin().get(),
        plinst_parameters_search.stream_connected()
            .map_ok(|p| async move {
                p.plugin_parameter().get().await.map(|pp| {
                    format!("{}={}", pp.object.flag, shlex_quote(&p.object.value))
                })
            })
            .try_buffered(threads)
            .try_collect()
    )?;
    let joined = if show_execshell {
        format!(
            "{} {} {} {}",
            &plugin.object.dock_image,
            shlex_quote(&plugin.object.execshell),
            shlex_quote(&format!("{}/{}", &plugin.object.selfpath, &plugin.object.selfexec)),
            flags.join(" ")
        )
    } else {
        format!(
            "{} {} {}",
            &plugin.object.dock_image,
            shlex_quote(&plugin.object.selfexec),
            flags.join(" ")
        )
    };
    Ok(joined)
}

/// Wrapper for [shlex::try_quote] which never fails. NUL characters are replaced.
fn shlex_quote(in_str: &str) -> String {
    shlex::try_quote(&in_str.replace('\0', "¡NUL!")).unwrap().to_string()
}

async fn get_all_plugin_instances(feed: &FeedRo) -> Result<Vec<PluginInstanceRo>> {
    let sb = feed.get_plugin_instances().page_limit(20).max_items(100);
    let search = sb.search();
    let count = search.get_count().await?;
    if count > 100 {
        bail!("Feed contains over 100 plugin instances.")
    }
    search.stream_connected().try_collect().await.map_err(eyre::Error::new)
    // maybe a progress bar would be nice if count > 20
}
