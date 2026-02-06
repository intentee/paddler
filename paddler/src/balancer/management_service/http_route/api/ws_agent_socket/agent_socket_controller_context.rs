use std::sync::Arc;

use log::error;
use log::info;

use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::balancer::embedding_sender_collection::EmbeddingSenderCollection;
use crate::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

pub struct AgentSocketControllerContext {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub agent_id: String,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    pub embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    pub generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    pub model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
}

impl Drop for AgentSocketControllerContext {
    fn drop(&mut self) {
        if let Err(err) = self
            .agent_controller_pool
            .remove_agent_controller(&self.agent_id)
        {
            error!("Failed to remove agent: {err}");
        }

        info!("Removed agent: {}", self.agent_id);
    }
}
