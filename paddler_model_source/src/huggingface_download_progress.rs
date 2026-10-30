use std::sync::Arc;

use hf_hub::progress::DownloadEvent;
use hf_hub::progress::FileProgress;
use hf_hub::progress::ProgressEvent;
use hf_hub::progress::ProgressHandler;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::model_download_status::ModelDownloadStatus;

pub struct HuggingFaceDownloadProgress {
    pub model_path: ModelPath,
    pub resolve_ended: CancellationToken,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl HuggingFaceDownloadProgress {
    fn apply(&self, download_event: &DownloadEvent) {
        match download_event {
            DownloadEvent::Start { total_bytes, .. } => {
                self.slot_aggregated_status.register_fix(
                    &AgentIssueFix::HuggingFaceStartedDownloading(self.model_path.clone()),
                );
                self.slot_aggregated_status
                    .set_download_status(ModelDownloadStatus::Downloading {
                        downloaded_bytes: 0,
                        model_path: self.model_path.model_path.clone(),
                        total_bytes: *total_bytes,
                    });
            }
            DownloadEvent::Progress { files } => {
                for FileProgress {
                    bytes_completed, ..
                } in files
                {
                    self.slot_aggregated_status
                        .set_downloaded_bytes(*bytes_completed);
                }
            }
            DownloadEvent::AggregateProgress {
                bytes_completed, ..
            } => {
                self.slot_aggregated_status
                    .set_downloaded_bytes(*bytes_completed);
            }
            DownloadEvent::Complete => {
                self.slot_aggregated_status
                    .set_download_status(ModelDownloadStatus::NotDownloading);
            }
        }
    }
}

impl ProgressHandler for HuggingFaceDownloadProgress {
    fn on_progress(&self, progress_event: &ProgressEvent) {
        if self.resolve_ended.is_cancelled() {
            return;
        }

        match progress_event {
            ProgressEvent::Download(download_event) => self.apply(download_event),
            ProgressEvent::Upload(_upload_event) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hf_hub::progress::DownloadEvent;
    use hf_hub::progress::FileProgress;
    use hf_hub::progress::FileStatus;
    use hf_hub::progress::ProgressEvent;
    use hf_hub::progress::ProgressHandler as _;
    use hf_hub::progress::UploadEvent;
    use tokio_util::sync::CancellationToken;

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
            resolve_ended: CancellationToken::new(),
            slot_aggregated_status: Arc::clone(status),
        }
    }

    fn download(download_event: DownloadEvent) -> ProgressEvent {
        ProgressEvent::Download(download_event)
    }

    fn start_of(total_bytes: u64) -> ProgressEvent {
        download(DownloadEvent::Start {
            total_files: 1,
            total_bytes,
        })
    }

    fn file_progress(bytes_completed: u64) -> ProgressEvent {
        download(DownloadEvent::Progress {
            files: vec![FileProgress {
                filename: "model.gguf".to_owned(),
                bytes_completed,
                total_bytes: 1000,
                status: FileStatus::InProgress,
            }],
        })
    }

    fn downloading(downloaded_bytes: u64) -> ModelDownloadStatus {
        ModelDownloadStatus::Downloading {
            downloaded_bytes,
            model_path: MODEL_PATH.to_owned(),
            total_bytes: 1000,
        }
    }

    #[test]
    fn starting_a_download_reports_its_size_and_model_path() {
        let status = Arc::new(SlotAggregatedStatus::new(2));

        downloading_progress(&status).on_progress(&start_of(1000));

        assert_eq!(
            status.make_snapshot().status.download_status,
            downloading(0)
        );
    }

    #[test]
    fn starting_a_download_clears_the_lock_issue_of_the_downloaded_model() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let progress = downloading_progress(&status);
        let lock_issue = AgentIssue::HuggingFaceCannotAcquireLock(HuggingFaceDownloadLock {
            lock_path: "/tmp/lock".to_owned(),
            model_path: progress.model_path.clone(),
        });

        status.register_desired_state_issue(lock_issue.clone());

        progress.on_progress(&start_of(1000));

        assert!(!status.has_issue(&lock_issue));
    }

    #[test]
    fn file_progress_reports_the_bytes_completed_so_far() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let progress = downloading_progress(&status);

        progress.on_progress(&start_of(1000));
        progress.on_progress(&file_progress(300));
        progress.on_progress(&file_progress(500));

        assert_eq!(
            status.make_snapshot().status.download_status,
            downloading(500)
        );
    }

    #[test]
    fn aggregate_progress_reports_the_bytes_completed_so_far() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let progress = downloading_progress(&status);

        progress.on_progress(&start_of(1000));
        progress.on_progress(&download(DownloadEvent::AggregateProgress {
            bytes_completed: 700,
            total_bytes: 1000,
            bytes_per_sec: None,
        }));

        assert_eq!(
            status.make_snapshot().status.download_status,
            downloading(700)
        );
    }

    #[test]
    fn completing_a_download_stops_reporting_it() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let progress = downloading_progress(&status);

        progress.on_progress(&start_of(1000));
        progress.on_progress(&download(DownloadEvent::Complete));

        assert_eq!(
            status.make_snapshot().status.download_status,
            ModelDownloadStatus::NotDownloading
        );
    }

    #[test]
    fn upload_events_leave_the_download_status_unchanged() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let progress = downloading_progress(&status);

        progress.on_progress(&start_of(1000));
        progress.on_progress(&ProgressEvent::Upload(UploadEvent::Committing));

        assert_eq!(
            status.make_snapshot().status.download_status,
            downloading(0)
        );
    }

    #[test]
    fn events_after_the_resolve_ended_leave_the_download_status_unchanged() {
        let status = Arc::new(SlotAggregatedStatus::new(2));
        let progress = downloading_progress(&status);

        progress.on_progress(&start_of(1000));
        progress.resolve_ended.cancel();
        progress.on_progress(&file_progress(300));

        assert_eq!(
            status.make_snapshot().status.download_status,
            downloading(0)
        );
    }
}
