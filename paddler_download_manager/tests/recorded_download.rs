use std::path::Path;

use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::download_outcome::DownloadOutcome;
use paddler_download_manager::download_progress::DownloadProgress;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub struct RecordedDownload {
    pub progress: Vec<DownloadProgress>,
    pub result: Result<DownloadOutcome, DownloadError>,
}

impl RecordedDownload {
    pub async fn download(
        cancellation_token: &CancellationToken,
        url: &str,
        final_path: &Path,
    ) -> Result<Self, reqwest::Error> {
        let recorded_progress = watch::Sender::new(Vec::new());
        let result = DownloadManager::new()?
            .download(cancellation_token, url, final_path, &|progress| {
                recorded_progress.send_modify(|recorded| recorded.push(progress));
            })
            .await;

        Ok(Self {
            progress: recorded_progress.send_replace(Vec::new()),
            result,
        })
    }

    pub fn written_byte_count(&self) -> u64 {
        self.progress
            .iter()
            .map(|progress| match progress {
                DownloadProgress::ChunkWritten { byte_count } => *byte_count,
                DownloadProgress::Finished | DownloadProgress::Started { .. } => 0,
            })
            .sum()
    }
}
