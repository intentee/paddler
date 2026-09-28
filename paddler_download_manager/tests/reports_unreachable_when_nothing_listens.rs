use paddler_download_manager::download_error::DownloadError;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn reports_unreachable_when_nothing_listens() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let unreachable_url = "http://127.0.0.1:1/never-listens";

    let recorded_download = RecordedDownload::download(
        &CancellationToken::new(),
        unreachable_url,
        &directory.path().join("model.gguf"),
    )
    .await
    .expect("the download manager must build its HTTP client");

    assert!(matches!(
        recorded_download.result,
        Err(DownloadError::DownloadServerIsUnreachable { url, .. }) if url == unreachable_url
    ));
}
