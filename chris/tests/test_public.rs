use bytes::Bytes;
use chris::models::Downloadable;
use chris::{types::*, AnonChrisClient, PublicChrisClient};
use futures::{future, pin_mut, StreamExt, TryFutureExt, TryStreamExt};
use rstest::*;

mod helpers;
use helpers::AnyResult;

const TESTING_URL: &str = "https://cube-for-testing-chrisui.apps.shift.nerc.mghpcc.org/api/v1/";

#[fixture]
fn cube_url() -> CubeUrl {
    TESTING_URL.to_string().parse().unwrap()
}

#[fixture]
#[once]
fn chris_client(cube_url: CubeUrl) -> AnonChrisClient {
    futures::executor::block_on(async { AnonChrisClient::connect(cube_url).await.unwrap() })
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_filebrowser_subdirs(chris_client: &AnonChrisClient) -> AnyResult {
    let fb = chris_client.filebrowser();
    let entry = fb
        .readdir("chrisui")
        .await?
        .expect("Filebrowser path not found");
    let subdirs = entry.subfolders();
    let expected_subdirs = ["feed_307", "feed_310"];
    for expected in expected_subdirs {
        assert!(subdirs.contains(&expected.to_string()))
    }
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_filebrowser_download_file(chris_client: &AnonChrisClient) -> AnyResult {
    let fb = chris_client.filebrowser();
    let entry = fb
        .readdir("chrisui/feed_310/pl-dircopy_313/pl-unstack-folders_314/pl-mri-preview_875/data")
        .await?
        .expect("Filebrowser path not found");
    let search = entry.iter_files();
    let search_results = search
        .stream_connected()
        .try_filter(|f| future::ready(f.data.fname().as_str().ends_with("/fetal-template-22.txt")));
    pin_mut!(search_results);
    let file = search_results
        .next()
        .await
        .expect("No files found in filebrowser path")?;
    let stream = file.stream().await?;
    let chunks: Vec<Bytes> = stream.try_collect().await?;
    let bytes: Vec<u8> = chunks.into_iter().flat_map(|chunk| chunk.into_iter()).collect();
    let actual = String::from_utf8(bytes)?;
    let expected = "1961680 voxels\n1245019.9508666992 mm^3".to_string();
    assert_eq!(actual, expected);
    Ok(())
}
