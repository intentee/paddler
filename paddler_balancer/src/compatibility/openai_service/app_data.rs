use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::buffered_request_manager::BufferedRequestManager;
use crate::inference_service::configuration::Configuration;

pub struct AppData {
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub inference_service_configuration: Configuration,
    pub shutdown: CancellationToken,
}
