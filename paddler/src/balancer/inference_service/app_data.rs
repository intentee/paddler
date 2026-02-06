use std::sync::Arc;

use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::inference_service::configuration::Configuration;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

pub struct AppData {
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub inference_service_configuration: Configuration,
}
