use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::embedding_sender_collection::EmbeddingSenderCollection;
use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::state_database::StateDatabase;

pub struct AppData {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    pub embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    pub generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    pub model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
    pub shutdown: CancellationToken,
    pub state_database: Arc<dyn StateDatabase>,
    pub statsd_prefix: String,
}
