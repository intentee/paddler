use tempfile::TempDir;
use tokio::fs::try_exists;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::download_outcome::DownloadOutcome;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn returns_cancelled_without_a_request_for_a_cancelled_token() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");
    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(b"body".to_vec()))
        .await
        .expect("the local HTTP fixture must start");
    let cancellation_token = CancellationToken::new();

    cancellation_token.cancel();

    let recorded_download = RecordedDownload::download(
        &download_manager,
        &cancellation_token,
        &fixture.url("/model.gguf"),
        &destination,
    )
    .await;

    assert_eq!(
        recorded_download
            .result
            .expect("a cancelled download must not fail"),
        DownloadOutcome::Cancelled
    );
    assert!(
        !try_exists(&destination)
            .await
            .expect("the destination must be inspectable")
    );
    assert_eq!(fixture.request_count(), 0);
}
