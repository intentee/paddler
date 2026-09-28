#![cfg(unix)]

use paddler_download_manager::download_error::DownloadError;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use tempfile::TempDir;
use tokio::fs::create_dir;
use tokio::fs::write;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_io_error_when_the_destination_is_a_non_empty_directory() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");

    create_dir(&destination)
        .await
        .expect("the destination directory must be creatable");
    write(destination.join("blocker"), b"x")
        .await
        .expect("the destination directory must accept a file");

    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(b"body".to_vec()))
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
        Err(DownloadError::Io { .. })
    ));
}
