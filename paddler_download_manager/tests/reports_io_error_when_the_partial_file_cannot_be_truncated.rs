#![cfg(unix)]

use paddler_download_manager::download_error::DownloadError;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use tempfile::TempDir;
use tokio::fs::create_dir;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_io_error_when_the_partial_file_cannot_be_truncated() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");

    create_dir(destination.with_extension("partial"))
        .await
        .expect("a directory must be creatable at the partial path");

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

    assert!(fixture.last_range_header().is_some());
    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::Io { .. })
    ));
}
