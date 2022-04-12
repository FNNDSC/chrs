use chris::api::AnyFilesUrl;
use chris::{CUBEError, ChrisClient};
use futures::Stream;
use std::future::Future;

pub(crate) async fn download(_client: &ChrisClient, _url: &AnyFilesUrl) -> anyhow::Result<()> {
    anyhow::Ok(())
}

// fn stream2download(
//     client: &ChrisClient, url: &AnyFilesUrl
// ) -> impl Stream<Item = impl Future<Output = Result<>>>{
//
// }
