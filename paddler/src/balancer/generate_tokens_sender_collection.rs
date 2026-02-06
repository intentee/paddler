use async_trait::async_trait;
use dashmap::DashMap;
use paddler_types::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

use crate::balancer::manages_senders::ManagesSenders;

pub struct GenerateTokensSenderCollection {
    senders: DashMap<String, mpsc::UnboundedSender<GeneratedTokenResult>>,
}

impl Default for GenerateTokensSenderCollection {
    fn default() -> Self {
        Self {
            senders: DashMap::new(),
        }
    }
}

#[async_trait]
impl ManagesSenders for GenerateTokensSenderCollection {
    type Value = GeneratedTokenResult;

    fn get_sender_collection(&self) -> &DashMap<String, mpsc::UnboundedSender<Self::Value>> {
        &self.senders
    }
}
