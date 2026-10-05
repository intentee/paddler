#![cfg(unix)]

use std::fs::Permissions;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt as _;

use tempfile::TempDir;
use tokio::fs::create_dir;
use tokio::fs::set_permissions;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_cache_permission_denied_when_the_cache_directory_is_read_only() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let read_only_directory = directory.path().join("read_only");

    create_dir(&read_only_directory)
        .await
        .expect("the cache directory must be creatable");
    set_permissions(&read_only_directory, Permissions::from_mode(0o500))
        .await
        .expect("the cache directory must become read-only");

    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(b"body".to_vec()))
        .await
        .expect("the local HTTP fixture must start");

    let recorded_download = RecordedDownload::download(
        &download_manager,
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &read_only_directory.join("model.gguf"),
    )
    .await;

    set_permissions(&read_only_directory, Permissions::from_mode(0o700))
        .await
        .expect("the cache directory permissions must be restorable");

    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::CachePermissionDenied { source, .. }) if source.kind() == ErrorKind::PermissionDenied
    ));
}
