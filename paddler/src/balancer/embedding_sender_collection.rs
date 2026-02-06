use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::mpsc;

use crate::balancer::manages_senders::ManagesSenders;
use crate::embedding_result::EmbeddingResult;

pub struct EmbeddingSenderCollection {
    senders: DashMap<String, mpsc::UnboundedSender<EmbeddingResult>>,
}

impl Default for EmbeddingSenderCollection {
    fn default() -> Self {
        Self {
            senders: DashMap::new(),
        }
    }
}

#[async_trait]
impl ManagesSenders for EmbeddingSenderCollection {
    type Value = EmbeddingResult;

    fn get_sender_collection(&self) -> &DashMap<String, mpsc::UnboundedSender<Self::Value>> {
        &self.senders
    }
}
