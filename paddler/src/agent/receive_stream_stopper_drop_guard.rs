use std::sync::Arc;

use log::error;

use crate::agent::receive_stream_stopper_collection::ReceiveStreamStopperCollection;

pub struct ReceiveStreamStopperDropGuard {
    pub receive_stream_stopper_collection: Arc<ReceiveStreamStopperCollection>,
    pub request_id: String,
}

impl Drop for ReceiveStreamStopperDropGuard {
    fn drop(&mut self) {
        if let Err(err) = self
            .receive_stream_stopper_collection
            .deregister_stopper(self.request_id.clone())
        {
            error!(
                "Failed to deregister stopper for request_id {}: {}",
                self.request_id, err
            );
        }
    }
}
