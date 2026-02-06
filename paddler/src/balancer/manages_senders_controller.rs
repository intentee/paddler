use std::sync::Arc;

use anyhow::Result;
use log::error;
use tokio::sync::mpsc;

use crate::balancer::manages_senders::ManagesSenders;

pub struct ManagesSendersController<TSenderCollection>
where
    TSenderCollection: ManagesSenders,
{
    pub request_id: String,
    pub response_rx: mpsc::UnboundedReceiver<TSenderCollection::Value>,
    pub response_sender_collection: Arc<TSenderCollection>,
}

impl<TSenderCollection> ManagesSendersController<TSenderCollection>
where
    TSenderCollection: ManagesSenders,
{
    pub fn from_request_id(
        request_id: String,
        response_sender_collection: Arc<TSenderCollection>,
    ) -> Result<Self> {
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        response_sender_collection.register_sender(request_id.clone(), response_tx)?;

        Ok(Self {
            request_id,
            response_rx,
            response_sender_collection,
        })
    }
}

impl<TSenderCollection> Drop for ManagesSendersController<TSenderCollection>
where
    TSenderCollection: ManagesSenders,
{
    fn drop(&mut self) {
        self.response_sender_collection
            .deregister_sender(self.request_id.clone())
            .unwrap_or_else(|err| {
                error!(
                    "Failed to deregister sender for request_id {}: {err}",
                    self.request_id
                );
            });
    }
}
