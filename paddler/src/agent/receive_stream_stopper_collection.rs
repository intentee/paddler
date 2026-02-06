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
    pub fn deregister_stopper(&self, request_id: String) -> Result<()> {
        if let Some(stopper) = self.receive_stoppers.remove(&request_id) {
            drop(stopper);

            Ok(())
        } else {
            Err(anyhow!("No stopper found for request_id {request_id}"))
        }
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

    pub fn stop(&self, request_id: String) -> Result<()> {
        if let Some(stopper) = self.receive_stoppers.get(&request_id) {
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
