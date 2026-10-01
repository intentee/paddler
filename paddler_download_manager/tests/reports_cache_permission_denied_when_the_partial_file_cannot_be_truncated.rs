#![cfg(unix)]

use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt as _;

use tempfile::TempDir;
use tokio::fs::set_permissions;
use tokio::fs::write;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_cache_permission_denied_when_the_partial_file_cannot_be_truncated() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");
    let partial_path = destination.with_extension("partial");

    write(&partial_path, b"partial")
        .await
        .expect("the partial file must be writable");
    set_permissions(&partial_path, Permissions::from_mode(0o400))
        .await
        .expect("the partial file must become read-only");

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

    assert!(fixture.last_range_header().is_some());
    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::CachePermissionDenied { path, .. }) if path == partial_path
    ));
}
