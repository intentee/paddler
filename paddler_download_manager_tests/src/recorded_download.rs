use std::path::Path;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::download_outcome::DownloadOutcome;
use paddler_download_manager::download_progress::DownloadProgress;
use paddler_download_manager::download_url::DownloadUrl;

pub struct RecordedDownload {
    pub progress: Vec<DownloadProgress>,
    pub result: Result<DownloadOutcome, DownloadError>,
}

impl RecordedDownload {
    pub async fn download(
        download_manager: &DownloadManager,
        cancellation_token: &CancellationToken,
        url: &str,
        final_path: &Path,
    ) -> Self {
        let recorded_progress = watch::Sender::new(Vec::new());
        let result = match DownloadUrl::parse(url) {
            Ok(download_url) => {
                download_manager
                    .download(cancellation_token, &download_url, final_path, &|progress| {
                        recorded_progress.send_modify(|recorded| recorded.push(progress));
                    })
                    .await
            }
            Err(url_error) => Err(url_error),
        };

        Self {
            progress: recorded_progress.send_replace(Vec::new()),
            result,
        }
    }

    #[must_use]
    pub fn downloaded_bytes_of_the_last_written_chunk(&self) -> Option<u64> {
        self.progress
            .iter()
            .rev()
            .find_map(|progress| match progress {
                DownloadProgress::ChunkWritten { downloaded_bytes } => Some(*downloaded_bytes),
                DownloadProgress::Finished | DownloadProgress::Started { .. } => None,
            })
    }
}
