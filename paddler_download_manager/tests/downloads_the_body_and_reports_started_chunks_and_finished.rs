use paddler_download_manager::download_outcome::DownloadOutcome;
use paddler_download_manager::download_progress::DownloadProgress;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use tempfile::TempDir;
use tokio::fs::read;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn downloads_the_body_and_reports_started_chunks_and_finished() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");
    let body = b"Hello, GGUF world!".to_vec();
    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(body.clone()))
        .await
        .expect("the local HTTP fixture must start");

    let recorded_download = RecordedDownload::download(
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &destination,
    )
    .await
    .expect("the download manager must build its HTTP client");

    assert_eq!(
        recorded_download
            .result
            .as_ref()
            .expect("the download must succeed"),
        &DownloadOutcome::Completed
    );
    assert_eq!(
        read(&destination)
            .await
            .expect("the downloaded file must be readable"),
        body
    );
    assert_eq!(
        recorded_download.progress.first(),
        Some(&DownloadProgress::Started {
            already_downloaded_bytes: 0,
            total_bytes: Some(body.len() as u64),
        })
    );
    assert_eq!(recorded_download.written_byte_count(), body.len() as u64);
    assert_eq!(
        recorded_download.progress.last(),
        Some(&DownloadProgress::Finished)
    );
}
