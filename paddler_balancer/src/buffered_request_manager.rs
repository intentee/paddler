use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use tokio::sync::watch;
use tokio::time::timeout;

use crate::agent_controller_pool::AgentControllerPool;
use crate::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
use crate::buffered_request_counter::BufferedRequestCounter;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

pub struct BufferedRequestManager {
    agent_controller_pool: Arc<AgentControllerPool>,
    pub buffered_request_counter: Arc<BufferedRequestCounter>,
    buffered_request_timeout: Duration,
    max_buffered_requests: i32,
    update_tx: watch::Sender<()>,
}

impl BufferedRequestManager {
    #[must_use]
    pub fn new(
        agent_controller_pool: Arc<AgentControllerPool>,
        buffered_request_timeout: Duration,
        max_buffered_requests: i32,
    ) -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            agent_controller_pool,
            buffered_request_counter: Arc::new(BufferedRequestCounter::new(update_tx.clone())),
            buffered_request_timeout,
            max_buffered_requests,
            update_tx,
        }
    }

    pub async fn wait_for_available_agent(&self) -> Result<BufferedRequestAgentWaitResult> {
        if let Some(dispatched_agent) = self
            .agent_controller_pool
            .take_least_busy_agent_controller()
        {
            return Ok(BufferedRequestAgentWaitResult::Found(dispatched_agent));
        }

        if self.buffered_request_counter.get() >= self.max_buffered_requests {
            return Ok(BufferedRequestAgentWaitResult::BufferOverflow);
        }

        let _buffered_request_count_guard = self.buffered_request_counter.increment_with_guard();
        let agent_controller_pool = self.agent_controller_pool.clone();
        let mut update_rx = agent_controller_pool.subscribe_to_updates();

        match timeout(self.buffered_request_timeout, async {
            loop {
                if let Some(dispatched_agent) =
                    agent_controller_pool.take_least_busy_agent_controller()
                {
                    return Ok::<_, anyhow::Error>(BufferedRequestAgentWaitResult::Found(
                        dispatched_agent,
                    ));
                }

                update_rx.changed().await?;
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

    fn make_snapshot(&self) -> Self::Snapshot {
        BufferedRequestManagerSnapshot {
            buffered_requests_current: self.buffered_request_counter.get(),
        }
    }
}

impl SubscribesToUpdates for BufferedRequestManager {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}
