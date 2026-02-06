use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::Notify;
use tokio::time::timeout;

use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
use crate::balancer::buffered_request_counter::BufferedRequestCounter;
use crate::balancer::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use crate::produces_snapshot::ProducesSnapshot;

pub struct BufferedRequestManager {
    agent_controller_pool: Arc<AgentControllerPool>,
    pub buffered_request_counter: Arc<BufferedRequestCounter>,
    buffered_request_timeout: Duration,
    max_buffered_requests: i32,
    pub update_notifier: Arc<Notify>,
}

impl BufferedRequestManager {
    pub fn new(
        agent_controller_pool: Arc<AgentControllerPool>,
        buffered_request_timeout: Duration,
        max_buffered_requests: i32,
    ) -> Self {
        let update_notifier = Arc::new(Notify::new());

        Self {
            agent_controller_pool,
            buffered_request_counter: Arc::new(BufferedRequestCounter::new(
                update_notifier.clone(),
            )),
            buffered_request_timeout,
            max_buffered_requests,
            update_notifier,
        }
    }

    pub async fn wait_for_available_agent(&self) -> Result<BufferedRequestAgentWaitResult> {
        if self.buffered_request_counter.get() >= self.max_buffered_requests {
            return Ok(BufferedRequestAgentWaitResult::BufferOverflow);
        }

        // Do a quick check before getting into the coroutines
        if let Some(agent_controller) = self
            .agent_controller_pool
            .take_least_busy_agent_controller()
        {
            return Ok(BufferedRequestAgentWaitResult::Found(agent_controller));
        }

        let _buffered_request_count_guard = self.buffered_request_counter.increment_with_guard();
        let agent_controller_pool = self.agent_controller_pool.clone();

        match timeout(self.buffered_request_timeout, async {
            loop {
                match agent_controller_pool.take_least_busy_agent_controller() {
                    Some(agent_controller) => {
                        return Ok::<_, anyhow::Error>(BufferedRequestAgentWaitResult::Found(
                            agent_controller,
                        ));
                    }
                    None => agent_controller_pool.update_notifier.notified().await,
                }
            }
        })
        .await
        {
            Ok(inner_result) => Ok(inner_result?),
            Err(timeout_err) => Ok(BufferedRequestAgentWaitResult::Timeout(timeout_err.into())),
        }
    }
}

impl ProducesSnapshot for BufferedRequestManager {
    type Snapshot = BufferedRequestManagerSnapshot;

    fn make_snapshot(&self) -> Result<Self::Snapshot> {
        Ok(BufferedRequestManagerSnapshot {
            buffered_requests_current: self.buffered_request_counter.get(),
        })
    }
}
