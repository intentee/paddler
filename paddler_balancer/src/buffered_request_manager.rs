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

async fn take_agent_when_one_becomes_available(
    agent_controller_pool: &AgentControllerPool,
    update_rx: &mut watch::Receiver<()>,
) -> BufferedRequestAgentWaitResult {
    loop {
        if let Some(dispatched_agent) = agent_controller_pool.take_least_busy_agent_controller() {
            return BufferedRequestAgentWaitResult::Found(dispatched_agent);
        }

        if let Err(pool_update_channel_closed) = update_rx.changed().await {
            return BufferedRequestAgentWaitResult::Timeout(pool_update_channel_closed.into());
        }
    }
}

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

    #[must_use]
    pub fn buffered_request_manager_snapshot(&self) -> BufferedRequestManagerSnapshot {
        BufferedRequestManagerSnapshot {
            buffered_requests_current: self.buffered_request_counter.get(),
        }
    }

    pub async fn wait_for_available_agent(&self) -> BufferedRequestAgentWaitResult {
        if let Some(dispatched_agent) = self
            .agent_controller_pool
            .take_least_busy_agent_controller()
        {
            return BufferedRequestAgentWaitResult::Found(dispatched_agent);
        }

        if self.buffered_request_counter.get() >= self.max_buffered_requests {
            return BufferedRequestAgentWaitResult::BufferOverflow;
        }

        let _buffered_request_count_guard = self.buffered_request_counter.increment_with_guard();
        let agent_controller_pool = self.agent_controller_pool.clone();
        let mut update_rx = agent_controller_pool.subscribe_to_updates();

        match timeout(
            self.buffered_request_timeout,
            take_agent_when_one_becomes_available(&agent_controller_pool, &mut update_rx),
        )
        .await
        {
            Ok(wait_result) => wait_result,
            Err(timeout_err) => BufferedRequestAgentWaitResult::Timeout(timeout_err.into()),
        }
    }
}

impl ProducesSnapshot for BufferedRequestManager {
    type Snapshot = BufferedRequestManagerSnapshot;

    fn make_snapshot(&self) -> Result<Self::Snapshot> {
        Ok(self.buffered_request_manager_snapshot())
    }
}

impl SubscribesToUpdates for BufferedRequestManager {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use std::collections::BTreeSet;
    use std::mem::Discriminant;
    use std::mem::discriminant;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;

    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::agent_controller::AgentController;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use paddler_messaging::atomic_value::AtomicValue;

    fn found_result_discriminant() -> Discriminant<BufferedRequestAgentWaitResult> {
        let pool = AgentControllerPool::default();
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
            id: "agent-discriminant".to_owned(),
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

        pool.register_agent_controller("agent-discriminant".to_owned(), agent)
            .unwrap();

        let dispatched_agent = pool.take_least_busy_agent_controller().unwrap();

        discriminant(&BufferedRequestAgentWaitResult::Found(dispatched_agent))
    }

    #[test]
    fn make_snapshot_reports_the_current_buffered_request_count() {
        let manager = BufferedRequestManager::new(
            Arc::new(AgentControllerPool::default()),
            Duration::ZERO,
            10,
        );

        manager.buffered_request_counter.increment();

        assert_eq!(
            manager.make_snapshot().unwrap().buffered_requests_current,
            1
        );
    }

    #[tokio::test]
    async fn counter_increment_wakes_subscribed_waiter() {
        let pool = Arc::new(AgentControllerPool::default());
        let manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_secs(1),
            10,
        ));

        let mut update_rx = manager.subscribe_to_updates();

        manager.buffered_request_counter.increment();

        let observed_within_deadline = timeout(Duration::from_secs(1), update_rx.changed())
            .await
            .unwrap();

        assert!(
            observed_within_deadline.is_ok(),
            "watch sender must stay alive while the manager holds it"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn waiter_returns_found_after_agent_registration_with_no_initial_agents() {
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

        pool.register_agent_controller("agent-1".to_owned(), agent)
            .unwrap();

        assert!(
            waiter.is_woken(),
            "register_agent_controller must wake the subscribed waiter"
        );

        let result = waiter.await;

        assert_eq!(
            discriminant(&result),
            found_result_discriminant(),
            "waiter must return Found after register_agent_controller"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn take_agent_reports_timeout_when_update_channel_closes() {
        let pool = AgentControllerPool::default();
        let (update_tx, mut update_rx) = watch::channel(());

        drop(update_tx);

        let result = take_agent_when_one_becomes_available(&pool, &mut update_rx).await;

        assert_eq!(
            discriminant(&result),
            discriminant(&BufferedRequestAgentWaitResult::Timeout(anyhow::anyhow!(
                "channel closed"
            ))),
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn waiter_times_out_when_no_agent_becomes_available() {
        let pool = Arc::new(AgentControllerPool::default());
        let manager = Arc::new(BufferedRequestManager::new(pool, Duration::ZERO, 10));

        let result = manager.wait_for_available_agent().await;

        assert_eq!(
            discriminant(&result),
            discriminant(&BufferedRequestAgentWaitResult::Timeout(anyhow::anyhow!(
                "deadline elapsed"
            ))),
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn waiter_returns_buffer_overflow_when_buffer_is_full() {
        let pool = Arc::new(AgentControllerPool::default());
        let manager = Arc::new(BufferedRequestManager::new(pool, Duration::from_mins(1), 0));

        let result = manager.wait_for_available_agent().await;

        assert_eq!(
            discriminant(&result),
            discriminant(&BufferedRequestAgentWaitResult::BufferOverflow),
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn waiter_returns_found_when_agent_was_registered_before_call() {
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

        pool.register_agent_controller("agent-pre".to_owned(), agent)
            .unwrap();

        let manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_mins(1),
            10,
        ));

        let result = manager.wait_for_available_agent().await;

        assert_eq!(
            discriminant(&result),
            found_result_discriminant(),
            "waiter must return Found when an agent is already in the pool"
        );
    }
}
