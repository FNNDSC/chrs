use crate::executor::do_with_progress;
use anyhow::Context;
use async_stream::stream;
use chris::api::{AnyFilesUrl, Downloadable};
use chris::ChrisClient;
use futures::Stream;
use std::future::Future;
use std::path::{Path, PathBuf};
use tokio::fs;

pub(crate) async fn download(client: &ChrisClient, url: &AnyFilesUrl) -> anyhow::Result<()> {
    let count = client.get_count(url.as_str()).await.with_context(|| {
        format!(
            "Could not get count of files from {} -- is it a files URL?",
            url
        )
    })?;
    let stream = stream2download(client, url);
    do_with_progress(stream, count as u64, false).await?;
    anyhow::Ok(())
}

fn stream2download<'a>(
    client: &'a ChrisClient,
    url: &'a AnyFilesUrl,
) -> impl Stream<Item = Result<impl Future<Output = Result<(), DownloadError>> + 'a, DownloadError>> + 'a
{
    stream! {
        for await page in client.iter_files(url) {
            yield match page {
                Err(e) => Err(DownloadError::Pagination(e)),
                Ok(downloadable) => {
                    Ok(download_helper(client, downloadable))
                }
            };
        }
    }
}

async fn download_helper(
    client: &ChrisClient,
    downloadable: impl Downloadable,
) -> Result<(), DownloadError> {
    let dst = Path::new(downloadable.fname().as_str());
    println!("Saving to {:?}", dst);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| DownloadError::ParentDirectory {
                path: parent.to_path_buf(),
                source: e,
            })?;
    }
    client
        .download_file(&downloadable, dst)
        .await
        .map_err(|e| e.into())
}

/// Errors which might occur when trying to download many files from
/// a collection URL.
#[derive(thiserror::Error, Debug)]
enum DownloadError {
    /// Error from paginating the given URL.
    #[error(transparent)]
    Pagination(#[from] reqwest::Error),

    /// Error from downloading from a `file_resource`.
    #[error(transparent)]
    Download(#[from] chris::FileIOError),

    #[error("Unable to create directory: {path:?}")]
    ParentDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
}
