use crate::login::UiUrl;
use crate::unicode;
use chris::{FeedResponse, FeedRo};
use color_eyre::owo_colors::OwoColorize;
use dialoguer::console::Term;
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

    let (styled_name, styled_id) = if feed.object.has_errored_job() {
        (
            name.bold().bright_red().to_string(),
            feed.object.id.0.bright_red().to_string(),
        )
    } else {
        (
            name.bold().bright_green().to_string(),
            feed.object.id.0.bright_green().to_string(),
        )
    };

    let id_part = format!("(feed/{})", styled_id);
    println!("{} {}  {}", symbol, styled_name, id_part.dimmed());
    if let Some(ui) = ui_url {
        println!("  {}", ui.feed_url_of(&feed.object))
    }
    let dim_lines = [
        "".to_string(),
        format!(
            "   created: {}",
            &feed.object.creation_date.format(&Rfc2822)?.italic()
        ),
        format!(
            "  modified: {}",
            &feed.object.modification_date.format(&Rfc2822)?.italic()
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

    let bar = "  |".dimmed();

    for dim_line in dim_lines {
        println!("{} {}", &bar, dim_line.dimmed())
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
        unicode::BLACK_DOWN_POINTING_TRIANGLE
            .bold()
            .red()
            .to_string()
    } else if feed.has_unfinished_jobs() {
        unicode::BLACK_UP_POINTING_TRIANGLE
            .bold()
            .yellow()
            .to_string()
    } else {
        unicode::BLACK_UP_POINTING_TRIANGLE
            .bold()
            .green()
            .to_string()
    }
}
