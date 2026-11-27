use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_request_registry::request_registry::RequestRegistry;

use crate::agent_controller_pool::AgentControllerPool;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;

pub struct InferenceSocketControllerContext {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub inference_service_configuration: InferenceServiceConfiguration,
    pub request_cancellation_tokens: Arc<RequestRegistry<CancellationToken>>,
    pub shutdown: CancellationToken,
}
