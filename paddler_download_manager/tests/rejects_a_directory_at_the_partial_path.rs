#![cfg(unix)]

use tempfile::TempDir;
use tokio::fs::create_dir;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn rejects_a_directory_at_the_partial_path() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");
    let partial_path = destination.with_extension("partial");

    create_dir(&partial_path)
        .await
        .expect("a directory must be creatable at the partial path");

    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(b"body".to_vec()))
        .await
        .expect("the local HTTP fixture must start");

    let recorded_download = RecordedDownload::download(
        &download_manager,
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &destination,
    )
    .await;

    assert!(fixture.last_range_header().is_none());
    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::PartialPathIsADirectory { partial_path: rejected_path })
            if rejected_path == partial_path
    ));
}
