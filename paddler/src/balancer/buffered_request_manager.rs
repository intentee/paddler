use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use paddler_types::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use tokio::sync::watch;
use tokio::time::timeout;

use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
use crate::balancer::buffered_request_counter::BufferedRequestCounter;
use crate::produces_snapshot::ProducesSnapshot;
use crate::subscribes_to_updates::SubscribesToUpdates;

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
        // Quick path: a slot is available right now, no buffering needed.
        if let Some(agent_controller) = self
            .agent_controller_pool
            .take_least_busy_agent_controller()
        {
            return Ok(BufferedRequestAgentWaitResult::Found(agent_controller));
        }

        // Slot is busy — we would need to wait. Reject if the buffer is full
        // (max_buffered_requests == 0 means buffering is disabled entirely).
        if self.buffered_request_counter.get() >= self.max_buffered_requests {
            return Ok(BufferedRequestAgentWaitResult::BufferOverflow);
        }

        let _buffered_request_count_guard = self.buffered_request_counter.increment_with_guard();
        let agent_controller_pool = self.agent_controller_pool.clone();
        let mut update_rx = agent_controller_pool.subscribe_to_updates();

        match timeout(self.buffered_request_timeout, async {
            loop {
                if let Some(agent_controller) =
                    agent_controller_pool.take_least_busy_agent_controller()
                {
                    return Ok::<_, anyhow::Error>(BufferedRequestAgentWaitResult::Found(
                        agent_controller,
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

    fn make_snapshot(&self) -> Result<Self::Snapshot> {
        Ok(BufferedRequestManagerSnapshot {
            buffered_requests_current: self.buffered_request_counter.get(),
        })
    }
}

impl SubscribesToUpdates for BufferedRequestManager {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn counter_increment_wakes_subscribed_waiter() -> Result<()> {
        let pool = Arc::new(AgentControllerPool::default());
        let manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_secs(1),
            10,
        ));

        let mut update_rx = manager.subscribe_to_updates();

        manager.buffered_request_counter.increment();

        timeout(Duration::from_secs(1), update_rx.changed())
            .await
            .map_err(|err| anyhow::anyhow!("subscriber did not observe within deadline: {err}"))?
            .map_err(|err| anyhow::anyhow!("watch sender dropped: {err}"))?;

        Ok(())
    }
}
