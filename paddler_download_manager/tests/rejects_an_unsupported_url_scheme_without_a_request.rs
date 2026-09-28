use paddler_download_manager::download_error::DownloadError;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn rejects_an_unsupported_url_scheme_without_a_request() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");

    let recorded_download = RecordedDownload::download(
        &CancellationToken::new(),
        "ftp://example.invalid/model.gguf",
        &directory.path().join("model.gguf"),
    )
    .await
    .expect("the download manager must build its HTTP client");

    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::UnsupportedUrlScheme { scheme, .. }) if scheme == "ftp"
    ));
}
