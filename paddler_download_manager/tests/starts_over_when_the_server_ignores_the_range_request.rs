use tempfile::TempDir;
use tokio::fs::read;
use tokio::fs::write;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::download_progress::DownloadProgress;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn starts_over_when_the_server_ignores_the_range_request() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");
    let body = b"fresh entire body".to_vec();

    write(
        destination.with_extension("partial"),
        b"stale partial bytes",
    )
    .await
    .expect("the partial file must be writable");

    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(body.clone()))
        .await
        .expect("the local HTTP fixture must start");

    let recorded_download = RecordedDownload::download(
        &download_manager,
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &destination,
    )
    .await;

    recorded_download
        .result
        .as_ref()
        .expect("the restarted download must succeed");

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
}
