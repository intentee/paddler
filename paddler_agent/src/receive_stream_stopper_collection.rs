use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use dashmap::DashMap;
use dashmap::DashSet;
use dashmap::mapref::entry::Entry;
use tokio::sync::mpsc;

use crate::receive_stream_stopper_drop_guard::ReceiveStreamStopperDropGuard;

pub struct ReceiveStreamStopperCollection {
    pending_stops: DashSet<String>,
    receive_stoppers: DashMap<String, mpsc::UnboundedSender<()>>,
}

impl ReceiveStreamStopperCollection {
    pub fn deregister_stopper(&self, request_id: &str) -> Result<()> {
        self.receive_stoppers.remove(request_id).map_or_else(
            || Err(anyhow!("No stopper found for request_id {request_id}")),
            |stopper| {
                drop(stopper);

                Ok(())
            },
        )
    }

    pub fn register_stopper(
        &self,
        request_id: String,
        stopper: mpsc::UnboundedSender<()>,
    ) -> Result<()> {
        match self.receive_stoppers.entry(request_id) {
            Entry::Occupied(occupied_stopper) => Err(anyhow!(
                "Stopper for request_id {} already exists",
                occupied_stopper.key()
            )),
            Entry::Vacant(vacant_stopper) => {
                if self.pending_stops.remove(vacant_stopper.key()).is_some() {
                    stopper.send(())?;
                }

                vacant_stopper.insert(stopper);

                Ok(())
            }
        }
    }

    pub fn register_stopper_with_guard(
        self: &Arc<Self>,
        request_id: String,
        stopper: mpsc::UnboundedSender<()>,
    ) -> Result<ReceiveStreamStopperDropGuard> {
        self.register_stopper(request_id.clone(), stopper)?;

        Ok(ReceiveStreamStopperDropGuard {
            receive_stream_stopper_collection: self.clone(),
            request_id,
        })
    }

    pub fn stop(&self, request_id: &str) -> Result<()> {
        if let Some(stopper) = self.receive_stoppers.get(request_id) {
            stopper.send(())?;
        } else {
            self.pending_stops.insert(request_id.to_owned());
        }

        Ok(())
    }
}

impl Default for ReceiveStreamStopperCollection {
    fn default() -> Self {
        Self {
            pending_stops: DashSet::new(),
            receive_stoppers: DashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_stopper_succeeds() {
        let collection = ReceiveStreamStopperCollection::default();
        let (sender, _receiver) = mpsc::unbounded_channel();

        assert!(
            collection
                .register_stopper("req_1".to_owned(), sender)
                .is_ok()
        );
    }

    #[test]
    fn register_duplicate_stopper_fails() {
        let collection = ReceiveStreamStopperCollection::default();
        let (sender_1, _receiver_1) = mpsc::unbounded_channel();
        let (sender_2, _receiver_2) = mpsc::unbounded_channel();

        collection
            .register_stopper("req_1".to_owned(), sender_1)
            .unwrap();

        assert!(
            collection
                .register_stopper("req_1".to_owned(), sender_2)
                .is_err()
        );
    }

    #[test]
    fn deregister_stopper_succeeds() {
        let collection = ReceiveStreamStopperCollection::default();
        let (sender, _receiver) = mpsc::unbounded_channel();

        collection
            .register_stopper("req_1".to_owned(), sender)
            .unwrap();

        assert!(collection.deregister_stopper("req_1").is_ok());
    }

    #[test]
    fn deregister_missing_stopper_fails() {
        let collection = ReceiveStreamStopperCollection::default();

        assert!(collection.deregister_stopper("nonexistent").is_err());
    }

    #[test]
    fn stop_sends_signal() {
        let collection = ReceiveStreamStopperCollection::default();
        let (sender, mut receiver) = mpsc::unbounded_channel();

        collection
            .register_stopper("req_1".to_owned(), sender)
            .unwrap();

        assert!(collection.stop("req_1").is_ok());
        assert!(receiver.try_recv().is_ok());
    }

    #[test]
    fn stop_arriving_before_registration_is_applied_when_the_request_registers() {
        let collection = ReceiveStreamStopperCollection::default();

        collection.stop("req_1").unwrap();

        let (sender, mut receiver) = mpsc::unbounded_channel();

        collection
            .register_stopper("req_1".to_owned(), sender)
            .unwrap();

        assert!(
            receiver.try_recv().is_ok(),
            "a stop that races ahead of the request must not be lost"
        );
    }

    #[test]
    fn stop_fails_when_receiver_dropped() {
        let collection = ReceiveStreamStopperCollection::default();
        let (sender, receiver) = mpsc::unbounded_channel();

        collection
            .register_stopper("req_1".to_owned(), sender)
            .unwrap();

        drop(receiver);

        assert!(collection.stop("req_1").is_err());
    }

    #[test]
    fn register_stopper_with_guard_fails_on_duplicate() {
        let collection = Arc::new(ReceiveStreamStopperCollection::default());
        let (sender_1, _receiver_1) = mpsc::unbounded_channel();
        let (sender_2, _receiver_2) = mpsc::unbounded_channel();

        collection
            .register_stopper("req_1".to_owned(), sender_1)
            .unwrap();

        assert!(
            collection
                .register_stopper_with_guard("req_1".to_owned(), sender_2)
                .is_err()
        );
    }

    #[test]
    fn register_stopper_with_guard_auto_deregisters_on_drop() {
        let collection = Arc::new(ReceiveStreamStopperCollection::default());
        let (sender, _receiver) = mpsc::unbounded_channel();

        let guard = collection
            .register_stopper_with_guard("req_1".to_owned(), sender)
            .unwrap();

        drop(guard);

        assert!(collection.deregister_stopper("req_1").is_err());
    }
}
