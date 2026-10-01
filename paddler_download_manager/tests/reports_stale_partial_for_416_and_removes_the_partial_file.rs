use http::StatusCode;
use tempfile::TempDir;
use tokio::fs::try_exists;
use tokio::fs::write;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_stale_partial_for_416_and_removes_the_partial_file() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");
    let partial_path = destination.with_extension("partial");

    write(&partial_path, b"stale")
        .await
        .expect("the partial file must be writable");

    let fixture =
        LocalHttpFixture::start(FixtureResponse::Status(StatusCode::RANGE_NOT_SATISFIABLE))
            .await
            .expect("the local HTTP fixture must start");

    let recorded_download = RecordedDownload::download(
        &download_manager,
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &destination,
    )
    .await;

    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::PartialFileStale { partial_path: stale_path, .. }) if stale_path == partial_path
    ));
    assert!(
        !try_exists(&partial_path)
            .await
            .expect("the partial path must be inspectable")
    );
}
