use std::io::ErrorKind;

use tempfile::TempDir;
use tokio::fs::write;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_io_error_when_the_cache_directory_cannot_be_created() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let blocking_file = directory.path().join("blocker");

    write(&blocking_file, b"i am a file, not a directory")
        .await
        .expect("the blocking file must be writable");

    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(b"body".to_vec()))
        .await
        .expect("the local HTTP fixture must start");

    let destination = blocking_file.join("subdir").join("model.gguf");

    let recorded_download = RecordedDownload::download(
        &download_manager,
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &destination,
    )
    .await;

    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::Io { path, source })
            if path == destination.with_extension("partial")
                && source.kind() == ErrorKind::NotADirectory
    ));
}
