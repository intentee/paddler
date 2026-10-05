use std::sync::Arc;

use paddler_bootstrap::bootstrap_error::BootstrapError;

pub struct HomeData {
    pub error: Option<Arc<BootstrapError>>,
}
