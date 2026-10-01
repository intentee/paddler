use std::sync::Arc;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_download_manager::download_progress::DownloadProgress;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::model_download_status::ModelDownloadStatus;

pub struct UrlDownloadProgress {
    pub model_path: ModelPath,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl UrlDownloadProgress {
    pub fn apply(&self, progress: DownloadProgress) {
        match progress {
            DownloadProgress::Started {
                already_downloaded_bytes,
                total_bytes,
            } => {
                self.slot_aggregated_status.set_download_status(
                    self.started_download_status(already_downloaded_bytes, total_bytes),
                );
                self.slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelDownloadStarted(
                        self.model_path.clone(),
                    ));
            }
            DownloadProgress::ChunkWritten { byte_count } => {
                self.slot_aggregated_status.add_downloaded_bytes(byte_count);
            }
            DownloadProgress::Finished => {
                self.slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelDownloadCompleted(
                        self.model_path.clone(),
                    ));
                self.slot_aggregated_status
                    .set_download_status(ModelDownloadStatus::NotDownloading);
            }
        }
    }

    fn started_download_status(
        &self,
        already_downloaded_bytes: u64,
        total_bytes: Option<u64>,
    ) -> ModelDownloadStatus {
        let model_path = self.model_path.model_path.clone();

        match total_bytes {
            Some(total_bytes) => ModelDownloadStatus::Downloading {
                downloaded_bytes: already_downloaded_bytes,
                model_path,
                total_bytes,
            },
            None => ModelDownloadStatus::DownloadingWithUnknownSize {
                downloaded_bytes: already_downloaded_bytes,
                model_path,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_download_manager::download_progress::DownloadProgress;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::model_download_status::ModelDownloadStatus;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;

    use crate::url_download_progress::UrlDownloadProgress;

    const TEST_URL: &str = "https://example.com/m.gguf";

    fn test_model_path() -> ModelPath {
        ModelPath {
            model_path: TEST_URL.to_owned(),
        }
    }

    fn url_download_progress(status: &Arc<SlotAggregatedStatus>) -> UrlDownloadProgress {
        UrlDownloadProgress {
            model_path: test_model_path(),
            slot_aggregated_status: Arc::clone(status),
        }
    }

    #[test]
    fn starting_a_download_of_known_size_reports_it_and_clears_the_matching_download_issue() {
        let status = Arc::new(SlotAggregatedStatus::new(1));

        status.register_issue(AgentIssue::DownloadInterrupted(test_model_path()));

        url_download_progress(&status).apply(DownloadProgress::Started {
            already_downloaded_bytes: 100,
            total_bytes: Some(500),
        });

        let snapshot = status.make_snapshot();

        assert_eq!(
            snapshot.status.download_status,
            ModelDownloadStatus::Downloading {
                downloaded_bytes: 100,
                model_path: TEST_URL.to_owned(),
                total_bytes: 500,
            }
        );
        assert!(snapshot.status.issues.is_empty());
    }

    #[test]
    fn starting_a_download_of_unknown_size_reports_it_without_a_total() {
        let status = Arc::new(SlotAggregatedStatus::new(1));

        url_download_progress(&status).apply(DownloadProgress::Started {
            already_downloaded_bytes: 0,
            total_bytes: None,
        });

        assert_eq!(
            status.make_snapshot().status.download_status,
            ModelDownloadStatus::DownloadingWithUnknownSize {
                downloaded_bytes: 0,
                model_path: TEST_URL.to_owned(),
            }
        );
    }

    #[test]
    fn written_chunks_accumulate_the_downloaded_bytes() {
        let status = Arc::new(SlotAggregatedStatus::new(1));
        let download_progress = url_download_progress(&status);

        download_progress.apply(DownloadProgress::Started {
            already_downloaded_bytes: 0,
            total_bytes: Some(1000),
        });
        download_progress.apply(DownloadProgress::ChunkWritten { byte_count: 250 });
        download_progress.apply(DownloadProgress::ChunkWritten { byte_count: 125 });

        assert_eq!(
            status.make_snapshot().status.download_status,
            ModelDownloadStatus::Downloading {
                downloaded_bytes: 375,
                model_path: TEST_URL.to_owned(),
                total_bytes: 1000,
            }
        );
    }

    #[test]
    fn finishing_a_download_stops_reporting_it_and_clears_the_matching_download_issue() {
        let status = Arc::new(SlotAggregatedStatus::new(1));
        let download_progress = url_download_progress(&status);

        download_progress.apply(DownloadProgress::Started {
            already_downloaded_bytes: 200,
            total_bytes: Some(500),
        });
        status.register_issue(AgentIssue::DownloadInterrupted(test_model_path()));
        download_progress.apply(DownloadProgress::Finished);

        let snapshot = status.make_snapshot();

        assert_eq!(
            snapshot.status.download_status,
            ModelDownloadStatus::NotDownloading
        );
        assert!(snapshot.status.issues.is_empty());
    }
}
