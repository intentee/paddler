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
            .deregister_stopper(&self.request_id)
        {
            error!(
                "Failed to deregister stopper for request_id {}: {}",
                self.request_id, err
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::*;

    #[test]
    fn drop_deregisters_registered_stopper() {
        let receive_stream_stopper_collection = Arc::new(ReceiveStreamStopperCollection::default());
        let (sender, _receiver) = mpsc::unbounded_channel();
        let guard = ReceiveStreamStopperDropGuard {
            receive_stream_stopper_collection: receive_stream_stopper_collection.clone(),
            request_id: "req_1".to_owned(),
        };

        receive_stream_stopper_collection
            .register_stopper("req_1".to_owned(), sender)
            .unwrap();

        drop(guard);

        assert!(
            receive_stream_stopper_collection
                .deregister_stopper("req_1")
                .is_err()
        );
    }

    #[test]
    fn drop_handles_already_deregistered_stopper() {
        let receive_stream_stopper_collection = Arc::new(ReceiveStreamStopperCollection::default());
        let (sender, _receiver) = mpsc::unbounded_channel();
        let guard = ReceiveStreamStopperDropGuard {
            receive_stream_stopper_collection: receive_stream_stopper_collection.clone(),
            request_id: "req_1".to_owned(),
        };

        receive_stream_stopper_collection
            .register_stopper("req_1".to_owned(), sender)
            .unwrap();
        receive_stream_stopper_collection
            .deregister_stopper("req_1")
            .unwrap();

        drop(guard);

        assert!(
            receive_stream_stopper_collection
                .deregister_stopper("req_1")
                .is_err()
        );
    }
}
