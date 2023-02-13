use crate::constants::NUM_THREADS;
use crate::files::fname_util::MaybeNamer;
use chris::models::Downloadable;
use chris::ChrisClient;
use futures::{pin_mut, StreamExt, TryStreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) async fn list_files(
    client: &ChrisClient,
    src: String,
    full: bool,
    depth: u16,
    mut namer: MaybeNamer,
) -> anyhow::Result<()> {
    if !full {
        eprintln!("WARNING: `chrs ls` without `--tree` can only show --full output.")
    }
    if depth != 2 {
        eprintln!("WARNING: `chrs ls` without `--tree` ignores the `--depth` option.")
    }

    // FIXME
    // - duplicate code with download.rs
    // - namer should be saved and passed along to the directory download function.
    // - the whole shebang of accepting arguments as a union-type of URL, plugin instance title,
    //   "renamed" human-readable fname-like, fname-like, or fname, should be consolidated into
    //   one helper which is used everywhere.
    let src = if !src.starts_with("http") {
        namer.translate(&src).await?
    } else {
        src.to_string()
    };

    let url = crate::files::fname_util::parse_src(&src, client.url());

    let namer = Arc::new(Mutex::new(namer));
    let stream = client
        .iter_files(&url)
        .map_ok(|file| {
            let arc = Arc::clone(&namer);
            async move {
                let mut namer = arc.lock().await;
                Ok(namer.rename(file.fname()).await)
            }
        })
        .try_buffered(NUM_THREADS);
    pin_mut!(stream);
    while let Some(next) = stream.next().await {
        match next {
            Ok(named_fname) => println!("{}", named_fname),
            Err(e) => anyhow::bail!(e),
        }
    }
    anyhow::Ok(())
}
