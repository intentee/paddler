use std::sync::Arc;

use crate::balancer::buffered_request_counter::BufferedRequestCounter;

pub struct BufferedRequestCountGuard {
    buffered_requests_counter: Arc<BufferedRequestCounter>,
}

impl BufferedRequestCountGuard {
    pub fn new(buffered_requests_counter: Arc<BufferedRequestCounter>) -> Self {
        BufferedRequestCountGuard {
            buffered_requests_counter,
        }
    }
}

impl Drop for BufferedRequestCountGuard {
    fn drop(&mut self) {
        self.buffered_requests_counter.decrement();
    }
}
