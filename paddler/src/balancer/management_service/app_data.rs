use std::sync::Arc;

use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::balancer::embedding_sender_collection::EmbeddingSenderCollection;
use crate::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::balancer::state_database::StateDatabase;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

pub struct AppData {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    pub embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    pub generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    pub model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
    pub state_database: Arc<dyn StateDatabase>,
    pub statsd_prefix: String,
}
