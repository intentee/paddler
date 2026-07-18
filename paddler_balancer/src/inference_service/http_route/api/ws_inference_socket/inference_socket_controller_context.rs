use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::request_cancellation_tokens::RequestCancellationTokens;

pub struct InferenceSocketControllerContext {
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub inference_service_configuration: InferenceServiceConfiguration,
    pub request_cancellation_tokens: Arc<RequestCancellationTokens>,
    pub shutdown: CancellationToken,
}
