use std::sync::Arc;

use tokio_util::sync::DropGuard;

#[derive(Clone)]
pub struct ConnectionKeepAlive {
    pub stop_guard: Arc<DropGuard>,
}
