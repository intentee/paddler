use tokio::sync::watch;

use paddler_messaging::inference_mode::InferenceMode;

use crate::inference_socket::reported_cluster_inference_mode::ReportedClusterInferenceMode;

pub struct ClusterInferenceModeBroadcaster {
    reported_cluster_inference_mode_tx: watch::Sender<ReportedClusterInferenceMode>,
}

impl ClusterInferenceModeBroadcaster {
    pub fn publish(&self, inference_mode: InferenceMode) {
        let reported = ReportedClusterInferenceMode::Reported(inference_mode);

        self.reported_cluster_inference_mode_tx
            .send_if_modified(|last_reported| {
                if *last_reported == reported {
                    return false;
                }

                *last_reported = reported;

                true
            });
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<ReportedClusterInferenceMode> {
        self.reported_cluster_inference_mode_tx.subscribe()
    }
}

impl Default for ClusterInferenceModeBroadcaster {
    fn default() -> Self {
        let (reported_cluster_inference_mode_tx, _initial_reported_cluster_inference_mode_rx) =
            watch::channel(ReportedClusterInferenceMode::NotYetReported);

        Self {
            reported_cluster_inference_mode_tx,
        }
    }
}
