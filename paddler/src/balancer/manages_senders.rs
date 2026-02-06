use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use dashmap::DashMap;
use log::warn;
use serde::Serialize;
use tokio::sync::mpsc;

#[async_trait]
pub trait ManagesSenders {
    type Value: Send + Serialize + Sync + 'static;

    fn get_sender_collection(&self) -> &DashMap<String, mpsc::UnboundedSender<Self::Value>>;

    fn deregister_sender(&self, request_id: String) -> Result<()> {
        let senders = self.get_sender_collection();

        if let Some(sender) = senders.remove(&request_id) {
            drop(sender);

            Ok(())
        } else {
            Err(anyhow!("No sender found for request_id {request_id}"))
        }
    }

    async fn forward_response(&self, request_id: String, value: Self::Value) -> Result<()> {
        let senders = self.get_sender_collection();

        if let Some(sender) = senders.get(&request_id) {
            sender.send(value)?;

            Ok(())
        } else {
            Err(anyhow!("No sender found for request_id {request_id}"))
        }
    }

    async fn forward_response_safe(&self, request_id: String, value: Self::Value) {
        if let Err(err) = self.forward_response(request_id, value).await {
            // Metadata might come in after awaiting connection is closed
            warn!("Error forwarding response: {err}");
        }
    }

    fn register_sender(
        &self,
        request_id: String,
        sender: mpsc::UnboundedSender<Self::Value>,
    ) -> Result<()> {
        let senders = self.get_sender_collection();

        if senders.contains_key(&request_id) {
            return Err(anyhow!("Sender for request_id {request_id} already exists"));
        }

        senders.insert(request_id, sender);

        Ok(())
    }
}
