use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use dashmap::DashMap;
use log::warn;
use serde::Serialize;
use tokio::sync::mpsc;

#[async_trait]
pub trait ManagesSenders: Send + Sync {
    type Value: Send + Serialize + Sync + 'static;

    fn get_sender_collection(&self) -> &DashMap<String, mpsc::UnboundedSender<Self::Value>>;

    fn deregister_sender(&self, request_id: String) -> Result<()> {
        let senders = self.get_sender_collection();

        senders.remove(&request_id).map_or_else(
            || Err(anyhow!("No sender found for request_id {request_id}")),
            |sender| {
                drop(sender);

                Ok(())
            },
        )
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

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::ManagesSenders;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;

    #[test]
    fn register_sender_rejects_duplicate_request_id() {
        let sender_collection = EmbeddingSenderCollection::default();
        let request_id = "duplicate-request".to_owned();
        let (first_sender, _first_receiver) = mpsc::unbounded_channel();
        let (second_sender, _second_receiver) = mpsc::unbounded_channel();

        sender_collection
            .register_sender(request_id.clone(), first_sender)
            .unwrap();

        let duplicate_error = sender_collection
            .register_sender(request_id, second_sender)
            .err()
            .unwrap();

        assert_eq!(
            duplicate_error.to_string(),
            "Sender for request_id duplicate-request already exists"
        );
    }
}
