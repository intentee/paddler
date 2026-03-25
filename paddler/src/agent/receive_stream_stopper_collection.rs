use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use dashmap::DashMap;
use tokio::sync::mpsc;

use crate::agent::receive_stream_stopper_drop_guard::ReceiveStreamStopperDropGuard;

pub struct ReceiveStreamStopperCollection {
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
        if self.receive_stoppers.contains_key(&request_id) {
            return Err(anyhow!(
                "Stopper for request_id {request_id} already exists"
            ));
        }

        self.receive_stoppers.insert(request_id, stopper);

        Ok(())
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

            Ok(())
        } else {
            Err(anyhow!("No stopper found for request_id {request_id}"))
        }
    }
}

impl Default for ReceiveStreamStopperCollection {
    fn default() -> Self {
        Self {
            receive_stoppers: DashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

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
    fn register_duplicate_stopper_fails() -> Result<()> {
        let collection = ReceiveStreamStopperCollection::default();
        let (sender_1, _receiver_1) = mpsc::unbounded_channel();
        let (sender_2, _receiver_2) = mpsc::unbounded_channel();

        collection.register_stopper("req_1".to_owned(), sender_1)?;

        assert!(
            collection
                .register_stopper("req_1".to_owned(), sender_2)
                .is_err()
        );

        Ok(())
    }

    #[test]
    fn deregister_stopper_succeeds() -> Result<()> {
        let collection = ReceiveStreamStopperCollection::default();
        let (sender, _receiver) = mpsc::unbounded_channel();

        collection.register_stopper("req_1".to_owned(), sender)?;

        assert!(collection.deregister_stopper("req_1").is_ok());

        Ok(())
    }

    #[test]
    fn deregister_missing_stopper_fails() {
        let collection = ReceiveStreamStopperCollection::default();

        assert!(collection.deregister_stopper("nonexistent").is_err());
    }

    #[test]
    fn stop_sends_signal() -> Result<()> {
        let collection = ReceiveStreamStopperCollection::default();
        let (sender, mut receiver) = mpsc::unbounded_channel();

        collection.register_stopper("req_1".to_owned(), sender)?;

        assert!(collection.stop("req_1").is_ok());
        assert!(receiver.try_recv().is_ok());

        Ok(())
    }

    #[test]
    fn stop_missing_stopper_fails() {
        let collection = ReceiveStreamStopperCollection::default();

        assert!(collection.stop("nonexistent").is_err());
    }

    #[test]
    fn register_stopper_with_guard_auto_deregisters_on_drop() -> Result<()> {
        let collection = Arc::new(ReceiveStreamStopperCollection::default());
        let (sender, _receiver) = mpsc::unbounded_channel();

        let guard = collection.register_stopper_with_guard("req_1".to_owned(), sender)?;

        drop(guard);

        // After drop, the stopper should be deregistered
        assert!(collection.deregister_stopper("req_1").is_err());

        Ok(())
    }
}
