#![cfg(target_os = "linux")]

use std::io::ErrorKind;
use std::os::unix::fs::symlink;

use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

use crate::recorded_download::RecordedDownload;

const BODY: &[u8] = b"this body will fail to write because /dev/full";

#[tokio::test]
async fn reports_cache_disk_full_when_the_written_body_cannot_be_flushed() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");

    symlink("/dev/full", destination.with_extension("partial"))
        .expect("the partial path must link to the full device");

    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(BODY.to_vec()))
        .await
        .expect("the local HTTP fixture must start");

    let recorded_download = RecordedDownload::download(
        &download_manager,
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &destination,
    )
    .await;

    assert!(matches!(
        &recorded_download.result,
        Err(DownloadError::CacheDiskFull { source, .. }) if source.kind() == ErrorKind::StorageFull
    ));
    assert_eq!(recorded_download.written_byte_count(), BODY.len() as u64);

    fixture
        .stop()
        .await
        .expect("the fixture task must not panic")
        .expect("the fixture must serve every request");
}
