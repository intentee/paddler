use std::sync::Arc;
use std::time::Duration;

use crate::balancer::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use anyhow::Result;
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
        if let Some(dispatched_agent) = self
            .agent_controller_pool
            .take_least_busy_agent_controller()
        {
            return Ok(BufferedRequestAgentWaitResult::Found(dispatched_agent));
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
    use std::collections::BTreeSet;
    use std::sync::RwLock;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;
    use std::task::Poll;

    use crate::agent_state_application_status::AgentStateApplicationStatus;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::atomic_value::AtomicValue;
    use crate::balancer::agent_controller::AgentController;
    use crate::balancer::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
    use crate::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::balancer::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;

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

    #[tokio::test(flavor = "current_thread")]
    async fn waiter_returns_found_after_agent_registration_with_no_initial_agents() -> Result<()> {
        let pool = Arc::new(AgentControllerPool::default());
        let manager = Arc::new(BufferedRequestManager::new(
            pool.clone(),
            Duration::from_mins(1),
            10,
        ));

        let mut waiter =
            tokio_test::task::spawn(async move { manager.wait_for_available_agent().await });

        assert!(
            waiter.poll().is_pending(),
            "waiter must be Pending while pool has no agents"
        );

        let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();
        let agent = Arc::new(AgentController {
            agent_message_tx,
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            connection_close: CancellationToken::new(),
            desired_slots_total: AtomicValue::<AtomicI32>::new(1),
            download_current: AtomicValue::<AtomicU64>::new(0),
            download_filename: RwLock::new(None),
            download_indeterminate: AtomicValue::<AtomicBool>::new(true),
            download_total: AtomicValue::<AtomicU64>::new(0),
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            id: "agent-1".to_owned(),
            issues: RwLock::new(BTreeSet::new()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
            model_path: RwLock::new(None),
            name: None,
            newest_update_version: AtomicValue::<AtomicI32>::new(0),
            slots_processing: AtomicValue::<AtomicI32>::new(0),
            slots_total: AtomicValue::<AtomicI32>::new(1),
            state_application_status_code: AtomicValue::<AtomicI32>::new(
                AgentStateApplicationStatus::Fresh as i32,
            ),
            uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
        });

        pool.register_agent_controller("agent-1".to_owned(), agent)?;

        assert!(
            waiter.is_woken(),
            "register_agent_controller must wake the subscribed waiter"
        );

        let Poll::Ready(result) = waiter.poll() else {
            anyhow::bail!("waiter must be Ready after register_agent_controller, got Pending");
        };

        if !matches!(result?, BufferedRequestAgentWaitResult::Found(_)) {
            anyhow::bail!("waiter must return Found after register_agent_controller");
        }

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn waiter_returns_found_when_agent_was_registered_before_call() -> Result<()> {
        let pool = Arc::new(AgentControllerPool::default());

        let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();
        let agent = Arc::new(AgentController {
            agent_message_tx,
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            connection_close: CancellationToken::new(),
            desired_slots_total: AtomicValue::<AtomicI32>::new(1),
            download_current: AtomicValue::<AtomicU64>::new(0),
            download_filename: RwLock::new(None),
            download_indeterminate: AtomicValue::<AtomicBool>::new(true),
            download_total: AtomicValue::<AtomicU64>::new(0),
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            id: "agent-pre".to_owned(),
            issues: RwLock::new(BTreeSet::new()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
            model_path: RwLock::new(None),
            name: None,
            newest_update_version: AtomicValue::<AtomicI32>::new(0),
            slots_processing: AtomicValue::<AtomicI32>::new(0),
            slots_total: AtomicValue::<AtomicI32>::new(1),
            state_application_status_code: AtomicValue::<AtomicI32>::new(
                AgentStateApplicationStatus::Fresh as i32,
            ),
            uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
        });

        pool.register_agent_controller("agent-pre".to_owned(), agent)?;

        let manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_mins(1),
            10,
        ));

        let mut waiter =
            tokio_test::task::spawn(async move { manager.wait_for_available_agent().await });

        let Poll::Ready(result) = waiter.poll() else {
            anyhow::bail!(
                "waiter must be Ready on first poll when agent was registered before call"
            );
        };

        if !matches!(result?, BufferedRequestAgentWaitResult::Found(_)) {
            anyhow::bail!("waiter must return Found when an agent is already in the pool");
        }

        Ok(())
    }
}
