use paddler_download_manager::download_error::DownloadError;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use tempfile::TempDir;
use tokio::fs::try_exists;
use tokio::fs::write;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn treats_a_mismatched_content_range_as_a_stale_partial_file() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");
    let partial_path = destination.with_extension("partial");

    write(&partial_path, b"first half ")
        .await
        .expect("the partial file must be writable");

    let fixture = LocalHttpFixture::start(FixtureResponse::PartialContent {
        body: b"second half".to_vec(),
        content_range: "bytes 999-1009/22".to_owned(),
    })
    .await
    .expect("the local HTTP fixture must start");

    let recorded_download = RecordedDownload::download(
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &destination,
    )
    .await
    .expect("the download manager must build its HTTP client");

    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::PartialFileStale { partial_path: stale_path, .. }) if stale_path == partial_path
    ));
    assert!(
        !try_exists(&partial_path)
            .await
            .expect("the partial path must be inspectable")
    );
    assert!(recorded_download.progress.is_empty());
}
