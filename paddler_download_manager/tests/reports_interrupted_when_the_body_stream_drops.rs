use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_progress::DownloadProgress;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_interrupted_when_the_body_stream_drops() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let fixture = LocalHttpFixture::start(FixtureResponse::TruncatedBody {
        sent_body: b"abcdef".to_vec(),
        withheld_byte_count: 10,
    })
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
        Err(DownloadError::DownloadInterrupted { .. })
    ));
    assert_eq!(fixture.request_count(), 1);
    assert!(recorded_download.written_byte_count() <= 6);
    assert_ne!(
        recorded_download.progress.last(),
        Some(&DownloadProgress::Finished)
    );
}
