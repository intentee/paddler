#![cfg(unix)]

use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt as _;

use http::StatusCode;
use paddler_download_manager::download_error::DownloadError;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use tempfile::TempDir;
use tokio::fs::create_dir;
use tokio::fs::set_permissions;
use tokio::fs::write;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_cache_permission_denied_when_a_stale_partial_cannot_be_removed() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let locked_directory = directory.path().join("locked");
    let destination = locked_directory.join("model.gguf");

    create_dir(&locked_directory)
        .await
        .expect("the cache directory must be creatable");
    write(destination.with_extension("partial"), b"stale")
        .await
        .expect("the partial file must be writable");
    set_permissions(&locked_directory, Permissions::from_mode(0o500))
        .await
        .expect("the cache directory must become read-only");

    let fixture =
        LocalHttpFixture::start(FixtureResponse::Status(StatusCode::RANGE_NOT_SATISFIABLE))
            .await
            .expect("the local HTTP fixture must start");

    let recorded_download = RecordedDownload::download(
        &CancellationToken::new(),
        &fixture.url("/model.gguf"),
        &destination,
    )
    .await
    .expect("the download manager must build its HTTP client");

    set_permissions(&locked_directory, Permissions::from_mode(0o700))
        .await
        .expect("the cache directory permissions must be restorable");

    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::CachePermissionDenied { .. })
    ));
}
