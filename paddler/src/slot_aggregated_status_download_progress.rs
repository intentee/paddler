use std::sync::Arc;

use hf_hub::api::tokio::Progress;
use paddler_types::agent_issue_params::ModelPath;

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

        self.slot_aggregated_status
            .set_download_status(0, size, Some(filename.to_owned()));
    }

    async fn update(&mut self, size: usize) {
        self.slot_aggregated_status.increment_download_current(size);
    }

    async fn finish(&mut self) {
        self.slot_aggregated_status.reset_download();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use hf_hub::api::tokio::Progress;
    use paddler_types::agent_issue::AgentIssue;
    use paddler_types::agent_issue_params::HuggingFaceDownloadLock;
    use paddler_types::agent_issue_params::ModelPath;

    use crate::produces_snapshot::ProducesSnapshot;
    use crate::slot_aggregated_status::SlotAggregatedStatus;
    use crate::slot_aggregated_status_download_progress::SlotAggregatedStatusDownloadProgress;

    #[tokio::test]
    async fn test_init_sets_download_status_and_registers_fix() -> Result<()> {
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

        let snapshot = status.make_snapshot()?;

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

        Ok(())
    }

    #[tokio::test]
    async fn test_update_increments_download_current() -> Result<()> {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let mut progress = SlotAggregatedStatusDownloadProgress::new(Arc::clone(&status));

        progress.init(1000, "model.gguf").await;
        progress.update(300).await;
        progress.update(200).await;

        let snapshot = status.make_snapshot()?;

        assert_eq!(snapshot.download_current, 500);
        assert_eq!(snapshot.download_total, 1000);

        Ok(())
    }

    #[tokio::test]
    async fn test_finish_resets_download() -> Result<()> {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let mut progress = SlotAggregatedStatusDownloadProgress::new(Arc::clone(&status));

        progress.init(1000, "model.gguf").await;
        progress.update(1000).await;
        progress.finish().await;

        let snapshot = status.make_snapshot()?;

        assert_eq!(snapshot.download_current, 0);
        assert_eq!(snapshot.download_total, 0);
        assert_eq!(snapshot.download_filename, None);

        Ok(())
    }
}
