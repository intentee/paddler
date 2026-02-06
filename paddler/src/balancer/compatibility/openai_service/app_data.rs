use std::sync::Arc;

use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::inference_service::configuration::Configuration;

pub struct AppData {
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub inference_service_configuration: Configuration,
}
