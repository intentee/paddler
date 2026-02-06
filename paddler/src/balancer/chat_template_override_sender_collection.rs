use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::mpsc;

use crate::balancer::manages_senders::ManagesSenders;
use crate::chat_template::ChatTemplate;

pub struct ChatTemplateOverrideSenderCollection {
    senders: DashMap<String, mpsc::UnboundedSender<Option<ChatTemplate>>>,
}

impl Default for ChatTemplateOverrideSenderCollection {
    fn default() -> Self {
        Self {
            senders: DashMap::new(),
        }
    }
}

#[async_trait]
impl ManagesSenders for ChatTemplateOverrideSenderCollection {
    type Value = Option<ChatTemplate>;

    fn get_sender_collection(&self) -> &DashMap<String, mpsc::UnboundedSender<Self::Value>> {
        &self.senders
    }
}
