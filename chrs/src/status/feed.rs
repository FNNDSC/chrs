use crate::login::UiUrl;
use crate::unicode;
use chris::{FeedResponse, FeedRo};
use dialoguer::console::{style, Term};
use std::fmt::Display;
use time::format_description::well_known::Rfc2822;

pub async fn only_print_feed_status(
    feed: &FeedRo,
    ui_url: Option<UiUrl>,
) -> color_eyre::Result<()> {
    let symbol = feed_symbol_for(&feed.object);
    let name = if feed.object.name.is_empty() {
        "(no name)"
    } else {
        feed.object.name.as_str()
    };

    let styled_name = if feed.object.has_errored_job() {
        style(name).bold().bright().red().to_string()
    } else {
        style(name).bold().bright().green().to_string()
    };

    println!("{} {}", symbol, styled_name);
    if let Some(ui) = ui_url {
        println!("  {}", ui.feed_url_of(&feed.object))
    }
    let dim_lines = [
        "".to_string(),
        format!(
            "   created: {}",
            style(&feed.object.creation_date.format(&Rfc2822)?).italic()
        ),
        format!(
            "  modified: {}",
            style(&feed.object.modification_date.format(&Rfc2822)?).italic()
        ),
        "".to_string(),
        format!(
            "  finished: {}  pending: {}  running: {}  errors: {}",
            &feed.object.finished_jobs,
            feed.object.pending_jobs(),
            feed.object.running_jobs(),
            &feed.object.errored_jobs
        ),
    ];

    let bar = style("  |").dim();

    for dim_line in dim_lines {
        println!("{} {}", &bar, style(dim_line).dim())
    }

    let note = feed.note().get().await?;
    if !note.is_empty() {
        let term_cols = std::cmp::min(Term::stdout().size().1, 120) as usize;
        println!("{}", &bar);
        for line in textwrap::wrap(note.object.content.as_str(), term_cols) {
            println!("{} {}", &bar, line)
        }
    }
    Ok(())
}

fn feed_symbol_for(feed: &FeedResponse) -> impl Display {
    if feed.has_errored_job() {
        style(unicode::BLACK_DOWN_POINTING_TRIANGLE).bold().red()
    } else if feed.has_unfinished_jobs() {
        style(unicode::BLACK_UP_POINTING_TRIANGLE).bold().yellow()
    } else {
        style(unicode::BLACK_UP_POINTING_TRIANGLE).bold().green()
    }
}
