use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_manager::DownloadManager;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn sends_no_range_header_for_a_fresh_download() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(b"body".to_vec()))
        .await
        .expect("the local HTTP fixture must start");

    RecordedDownload::download(
        &download_manager,
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &directory.path().join("model.gguf"),
    )
    .await
    .result
    .expect("the download must succeed");

    assert_eq!(fixture.request_count(), 1);
    assert_eq!(fixture.last_range_header(), None);
}
