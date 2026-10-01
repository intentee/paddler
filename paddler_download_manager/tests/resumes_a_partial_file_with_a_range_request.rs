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
async fn resumes_a_partial_file_with_a_range_request() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");

    write(destination.with_extension("partial"), b"first half ")
        .await
        .expect("the partial file must be writable");

    let fixture = LocalHttpFixture::start(FixtureResponse::PartialContent {
        body: b"second half".to_vec(),
        content_range: "bytes 11-21/22".to_owned(),
    })
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
        .expect("the resumed download must succeed");

    assert_eq!(
        read(&destination)
            .await
            .expect("the downloaded file must be readable"),
        b"first half second half"
    );
    assert_eq!(fixture.last_range_header(), Some(b"bytes=11-".to_vec()));
    assert_eq!(
        recorded_download.progress.first(),
        Some(&DownloadProgress::Started {
            already_downloaded_bytes: 11,
            total_bytes: Some(22),
        })
    );
}
