use std::num::NonZeroUsize;

use log::debug;
use parking_lot::Mutex;
use tokio::sync::broadcast;

use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::inference_mode::InferenceMode;

use crate::inference_socket::reported_cluster_inference_mode::ReportedClusterInferenceMode;

pub struct ClusterInferenceModeBroadcaster {
    last_reported: Mutex<ReportedClusterInferenceMode>,
    notification_tx: broadcast::Sender<Notification>,
}

impl ClusterInferenceModeBroadcaster {
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> Self {
        let (notification_tx, _initial_notification_rx) = broadcast::channel(capacity.get());

        Self {
            last_reported: Mutex::new(ReportedClusterInferenceMode::NotYetReported),
            notification_tx,
        }
    }

    pub fn publish(&self, inference_mode: InferenceMode) {
        let reported = ReportedClusterInferenceMode::Reported(inference_mode);
        let mut last_reported = self.last_reported.lock();

        if *last_reported == reported {
            return;
        }

        *last_reported = reported;

        if self
            .notification_tx
            .send(Notification::ClusterInferenceMode(inference_mode))
            .is_err()
        {
            debug!("Dropped the cluster inference mode: no active subscribers");
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.notification_tx.subscribe()
    }
}
