use std::fmt::Display;

use color_eyre::eyre;
use color_eyre::eyre::{bail, eyre, Result};
use color_eyre::owo_colors::OwoColorize;
use dialoguer::console::Term;
use futures::TryStreamExt;
use itertools::Itertools;
use tokio::try_join;

use chris::errors::CubeError;
use chris::types::{PluginParameterAction, PluginParameterValue, SimplifiedStatus};
use chris::{FeedRo, PluginInstanceRo, PluginParameter, PluginRo};

use crate::login::UiUrl;
use crate::shlex::shlex_quote;
use crate::unicode;

use super::feed::only_print_feed_status;
use super::find_branch::find_branch_to;

pub async fn print_branch_status(
    feed: FeedRo,
    selected: PluginInstanceRo,
    ui_url: Option<UiUrl>,
    show_execshell: bool,
) -> Result<()> {
    only_print_feed_status(&feed, ui_url).await?;
    let all_plinst = get_all_plugin_instances(&feed).await?;
    let branch = find_branch_to(*selected.object.id, &all_plinst).ok_or_else(|| {
        eyre!(
            "plugininstance/{} not found in feed, which contains plugin instances {}",
            *selected.object.id,
            all_plinst.iter().map(|p| p.object.id.0).join(",")
        )
    })?;

    println!("\n{}", unicode::HORIZONTAL_BAR.repeat(40).dimmed());

    let term_cols = std::cmp::min(Term::stdout().size().1, 120) as usize;
    let branch_len = branch.len();
    for (i, plinst) in branch.into_iter().enumerate() {
        let is_current = plinst.object.id == selected.object.id;
        let has_next = i + 1 < branch_len;
        let id_part = format!("(plugininstance/{})", plinst.object.id.0.cyan());
        println!(
            "{} {}  {}",
            symbol_for(plinst),
            title_of(plinst, is_current),
            id_part.dimmed()
        );
        let pipe = if has_next { unicode::VERTICAL_BAR } else { " " };
        let cmd = cmd_of(plinst, show_execshell).await?;
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
        SimplifiedStatus::Running => unicode::BLACK_CIRCLE.bold().bright_blue().to_string(),
        SimplifiedStatus::Success => unicode::BLACK_CIRCLE.bold().blue().to_string(),
        SimplifiedStatus::Error => unicode::BLACK_CIRCLE.bold().bright_red().to_string(),
        SimplifiedStatus::Cancelled => unicode::WHITE_CIRCLE.dimmed().to_string(),
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

async fn cmd_of(plinst: &PluginInstanceRo, show_execshell: bool) -> Result<String, CubeError> {
    let plinst_parameters = plinst.parameters();
    let plinst_parameters_search = plinst_parameters.search();
    let (plugin, flags): (PluginRo, Vec<_>) = try_join!(
        plinst.plugin().get(),
        plinst_parameters_search
            .stream_connected()
            .try_filter_map(|p| async move {
                p.plugin_parameter()
                    .get()
                    .await
                    .map(|pp| format_param(pp.object, p.object.value))
            })
            .try_collect()
    )?;
    let joined = if show_execshell {
        format!(
            "{} {} {} {}",
            &plugin.object.dock_image,
            shlex_quote(&plugin.object.execshell),
            shlex_quote(&format!(
                "{}/{}",
                &plugin.object.selfpath, &plugin.object.selfexec
            )),
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

/// Somewhat equivalent to
/// https://github.com/FNNDSC/ChRIS_ultron_backEnd/blob/01b2928f65738d4266d210d80dc02eba3e530b20/chris_backend/plugininstances/services/manager.py#L399-L405
fn format_param(param: PluginParameter, value: PluginParameterValue) -> Option<String> {
    match param.action {
        PluginParameterAction::Store => Some(format!(
            "{}={}",
            param.flag,
            shlex_quote(value.to_string().as_str())
        )),
        PluginParameterAction::StoreTrue => {
            if let PluginParameterValue::Boolean(b) = value {
                if b {
                    Some(param.flag)
                } else {
                    None
                }
            } else {
                Some(format!(
                    "{}={} (invalid boolean value)",
                    param.flag,
                    shlex_quote(value.to_string().as_str())
                ))
            }
        }
        PluginParameterAction::StoreFalse => {
            if let PluginParameterValue::Boolean(b) = value {
                if b {
                    None
                } else {
                    Some(param.flag)
                }
            } else {
                Some(format!(
                    "{}={} (invalid boolean value)",
                    param.flag,
                    shlex_quote(value.to_string().as_str())
                ))
            }
        }
    }
}

async fn get_all_plugin_instances(feed: &FeedRo) -> Result<Vec<PluginInstanceRo>> {
    let sb = feed.get_plugin_instances().page_limit(20).max_items(100);
    let search = sb.search();
    let count = search.get_count().await?;
    if count > 100 {
        bail!("Feed contains over 100 plugin instances.")
    }
    search
        .stream_connected()
        .try_collect()
        .await
        .map_err(eyre::Error::new)
    // maybe a progress bar would be nice if count > 20
}
