use tempfile::TempDir;
use tokio::fs::read;
use tokio::fs::write;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::download_progress::DownloadProgress;
use paddler_download_manager_tests::recorded_download::RecordedDownload;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

#[tokio::test]
async fn rejects_a_declared_content_length_that_overflows_the_total_size() {
    let download_manager =
        DownloadManager::new().expect("the download manager must build its HTTP client");
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");
    let partial_path = destination.with_extension("partial");

    write(&partial_path, b"first half ")
        .await
        .expect("the partial file must be writable");

    let fixture = LocalHttpFixture::start(FixtureResponse::PartialContentDeclaringLength {
        content_range: "bytes 11-21/22".to_owned(),
        declared_content_length: 18_446_744_073_709_551_613,
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

    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::DeclaredContentLengthOverflowsTotalSize {
            already_downloaded_bytes,
            content_length,
            ..
        }) if already_downloaded_bytes == 11 && content_length == 18_446_744_073_709_551_613
    ));
    assert_eq!(
        read(&partial_path)
            .await
            .expect("the partial file must still be readable"),
        b"first half "
    );
    assert_eq!(recorded_download.progress, Vec::<DownloadProgress>::new());

    fixture
        .stop()
        .await
        .expect("the fixture task must not panic")
        .expect("the fixture must serve every request");
}
