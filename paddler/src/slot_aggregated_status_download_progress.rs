use std::sync::Arc;

use hf_hub::api::tokio::Progress;

use crate::agent_issue_fix::AgentIssueFix;
use crate::slot_aggregated_status::SlotAggregatedStatus;

#[derive(Clone)]
pub struct SlotAggregatedStatusDownloadProgress {
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl SlotAggregatedStatusDownloadProgress {
    pub fn new(slot_aggregated_status: Arc<SlotAggregatedStatus>) -> Self {
        Self {
            slot_aggregated_status,
        }
    }
}

impl Progress for SlotAggregatedStatusDownloadProgress {
    async fn init(&mut self, size: usize, filename: &str) {
        self.slot_aggregated_status
            .register_fix(AgentIssueFix::HuggingFaceStartedDownloading);

        self.slot_aggregated_status
            .set_download_status(0, size, Some(filename.to_string()));
    }

    async fn update(&mut self, size: usize) {
        self.slot_aggregated_status.increment_download_current(size);
    }

    async fn finish(&mut self) {
        self.slot_aggregated_status.reset_download();
    }
}
