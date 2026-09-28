use paddler_download_manager::download_error::DownloadError;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_unreachable_when_the_server_stalls_before_headers() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let fixture = LocalHttpFixture::start(FixtureResponse::StallBeforeHeaders)
        .await
        .expect("the local HTTP fixture must start");

    let recorded_download = RecordedDownload::download(
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &directory.path().join("model.gguf"),
    )
    .await
    .expect("the download manager must build its HTTP client");

    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::DownloadServerIsUnreachable { .. })
    ));
    assert!(recorded_download.progress.is_empty());
}
