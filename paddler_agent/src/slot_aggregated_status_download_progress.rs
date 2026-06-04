use std::sync::Arc;

use hf_hub::api::tokio::Progress;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::agent_issue_fix::AgentIssueFix;
use crate::slot_aggregated_status::SlotAggregatedStatus;

#[derive(Clone)]
pub struct SlotAggregatedStatusDownloadProgress {
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl SlotAggregatedStatusDownloadProgress {
    pub const fn new(slot_aggregated_status: Arc<SlotAggregatedStatus>) -> Self {
        Self {
            slot_aggregated_status,
        }
    }
}

impl Progress for SlotAggregatedStatusDownloadProgress {
    async fn init(&mut self, size: usize, filename: &str) {
        self.slot_aggregated_status
            .register_fix(&AgentIssueFix::HuggingFaceStartedDownloading(ModelPath {
                model_path: filename.to_owned(),
            }));

        self.slot_aggregated_status.set_download_status(
            0,
            Some(size as u64),
            Some(filename.to_owned()),
        );
    }

    async fn update(&mut self, size: usize) {
        self.slot_aggregated_status
            .increment_download_current(size as u64);
    }

    async fn finish(&mut self) {
        self.slot_aggregated_status.reset_download();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hf_hub::api::tokio::Progress;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;

    use crate::slot_aggregated_status::SlotAggregatedStatus;
    use crate::slot_aggregated_status_download_progress::SlotAggregatedStatusDownloadProgress;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;

    #[tokio::test]
    async fn test_init_sets_download_status_and_registers_fix() {
        let status = Arc::new(SlotAggregatedStatus::new(2));

        status.register_issue(AgentIssue::HuggingFaceCannotAcquireLock(
            HuggingFaceDownloadLock {
                lock_path: "/tmp/lock".to_owned(),
                model_path: ModelPath {
                    model_path: "model.gguf".to_owned(),
                },
            },
        ));

        let mut progress = SlotAggregatedStatusDownloadProgress::new(Arc::clone(&status));

        progress.init(1000, "model.gguf").await;

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.download_total, 1000);
        assert_eq!(snapshot.download_current, 0);
        assert_eq!(snapshot.download_filename, Some("model.gguf".to_owned()));
        assert!(!status.has_issue(&AgentIssue::HuggingFaceCannotAcquireLock(
            HuggingFaceDownloadLock {
                lock_path: "/tmp/lock".to_owned(),
                model_path: ModelPath {
                    model_path: "model.gguf".to_owned(),
                },
            },
        )));
    }

    #[tokio::test]
    async fn test_update_increments_download_current() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let mut progress = SlotAggregatedStatusDownloadProgress::new(Arc::clone(&status));

        progress.init(1000, "model.gguf").await;
        progress.update(300).await;
        progress.update(200).await;

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.download_current, 500);
        assert_eq!(snapshot.download_total, 1000);
    }

    #[tokio::test]
    async fn test_finish_resets_download() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let mut progress = SlotAggregatedStatusDownloadProgress::new(Arc::clone(&status));

        progress.init(1000, "model.gguf").await;
        progress.update(1000).await;
        progress.finish().await;

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.download_current, 0);
        assert_eq!(snapshot.download_total, 0);
        assert_eq!(snapshot.download_filename, None);
    }
}
