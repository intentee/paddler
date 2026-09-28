use std::sync::Arc;

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::embedding_sender_collection::EmbeddingSenderCollection;
use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;

pub struct AgentSocketControllerContext {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub agent_id: String,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    pub embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    pub generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    pub model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
}
