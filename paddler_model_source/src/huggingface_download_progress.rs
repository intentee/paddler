use std::sync::Arc;

use hf_hub::api::tokio::Progress;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::model_download_status::ModelDownloadStatus;

#[derive(Clone)]
pub struct HuggingFaceDownloadProgress {
    pub model_path: ModelPath,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl Progress for HuggingFaceDownloadProgress {
    async fn init(&mut self, size: usize, _filename_in_repo: &str) {
        self.slot_aggregated_status
            .register_fix(&AgentIssueFix::HuggingFaceStartedDownloading(
                self.model_path.clone(),
            ));

        self.slot_aggregated_status
            .set_download_status(ModelDownloadStatus::Downloading {
                downloaded_bytes: 0,
                model_path: self.model_path.model_path.clone(),
                total_bytes: size as u64,
            });
    }

    async fn update(&mut self, size: usize) {
        self.slot_aggregated_status
            .add_downloaded_bytes(size as u64);
    }

    async fn finish(&mut self) {
        self.slot_aggregated_status
            .set_download_status(ModelDownloadStatus::NotDownloading);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hf_hub::api::tokio::Progress;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::model_download_status::ModelDownloadStatus;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;

    use crate::huggingface_download_progress::HuggingFaceDownloadProgress;

    const MODEL_PATH: &str = "owner/repo/main/model.gguf";

    fn downloading_progress(status: &Arc<SlotAggregatedStatus>) -> HuggingFaceDownloadProgress {
        HuggingFaceDownloadProgress {
            model_path: ModelPath {
                model_path: MODEL_PATH.to_owned(),
            },
            slot_aggregated_status: Arc::clone(status),
        }
    }

    #[tokio::test]
    async fn starting_a_download_reports_its_size_and_model_path() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let mut progress = downloading_progress(&status);

        progress.init(1000, "model.gguf").await;

        assert_eq!(
            status.make_snapshot().status.download_status,
            ModelDownloadStatus::Downloading {
                downloaded_bytes: 0,
                model_path: MODEL_PATH.to_owned(),
                total_bytes: 1000,
            }
        );
    }

    #[tokio::test]
    async fn starting_a_download_clears_the_lock_issue_of_the_downloaded_model() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let mut progress = downloading_progress(&status);
        let lock_issue = AgentIssue::HuggingFaceCannotAcquireLock(HuggingFaceDownloadLock {
            lock_path: "/tmp/lock".to_owned(),
            model_path: progress.model_path.clone(),
        });

        status.register_issue(lock_issue.clone());

        progress.init(1000, "model.gguf").await;

        assert!(!status.has_issue(&lock_issue));
    }

    #[tokio::test]
    async fn progress_accumulates_the_downloaded_bytes() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let mut progress = downloading_progress(&status);

        progress.init(1000, "model.gguf").await;
        progress.update(300).await;
        progress.update(200).await;

        assert_eq!(
            status.make_snapshot().status.download_status,
            ModelDownloadStatus::Downloading {
                downloaded_bytes: 500,
                model_path: MODEL_PATH.to_owned(),
                total_bytes: 1000,
            }
        );
    }

    #[tokio::test]
    async fn finishing_a_download_stops_reporting_it() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let mut progress = downloading_progress(&status);

        progress.init(1000, "model.gguf").await;
        progress.update(1000).await;
        progress.finish().await;

        assert_eq!(
            status.make_snapshot().status.download_status,
            ModelDownloadStatus::NotDownloading
        );
    }
}
