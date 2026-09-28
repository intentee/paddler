#![cfg(target_os = "linux")]

use std::io::ErrorKind;
use std::os::unix::fs::symlink;

use paddler_download_manager::download_error::DownloadError;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_cache_disk_full_when_the_partial_file_is_on_a_full_device() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");

    symlink("/dev/full", destination.with_extension("partial"))
        .expect("the partial path must link to the full device");

    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(
        b"this body will fail to write because /dev/full".to_vec(),
    ))
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
        Err(DownloadError::CacheDiskFull { source, .. }) if source.kind() == ErrorKind::StorageFull
    ));
}
