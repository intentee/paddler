use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use dashmap::DashMap;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use tokio::sync::watch;

use super::agent_controller::AgentController;
use super::agent_controller_pool_total_slots::AgentControllerPoolTotalSlots;
use crate::agent_controller_slot_guard::AgentControllerSlotGuard;
use crate::dispatch_candidate::DispatchCandidate;
use crate::dispatched_agent::DispatchedAgent;
use crate::sets_desired_state::SetsDesiredState;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

pub struct AgentControllerPool {
    pub agents: DashMap<String, Arc<AgentController>>,
    update_tx: watch::Sender<()>,
}

impl AgentControllerPool {
    #[must_use]
    pub fn select_least_busy_with_capacity(&self) -> Option<DispatchCandidate> {
        let mut best: Option<DispatchCandidate> = None;

        for entry in &self.agents {
            let agent_controller = entry.value().clone();
            let snapshot = agent_controller.slots_processing.get();

            if snapshot >= agent_controller.slots_total.get() {
                continue;
            }

            best = Some(match best {
                Some(current) if current.snapshot <= snapshot => current,
                _ => DispatchCandidate {
                    agent_controller,
                    snapshot,
                },
            });
        }

        best
    }

    pub fn try_claim(
        &self,
        candidate: DispatchCandidate,
    ) -> Result<DispatchedAgent, DispatchCandidate> {
        if candidate
            .agent_controller
            .slots_processing
            .compare_and_swap(candidate.snapshot, candidate.snapshot + 1)
        {
            self.update_tx.send_replace(());

            let slot_guard = AgentControllerSlotGuard::new(
                candidate.agent_controller.clone(),
                self.update_tx.clone(),
            );

            Ok(DispatchedAgent::new(candidate.agent_controller, slot_guard))
        } else {
            Err(candidate)
        }
    }

    #[must_use]
    pub fn take_least_busy_agent_controller(&self) -> Option<DispatchedAgent> {
        loop {
            let candidate = self.select_least_busy_with_capacity()?;

            if let Ok(dispatched) = self.try_claim(candidate) {
                return Some(dispatched);
            }
        }
    }

    #[must_use]
    pub fn get_agent_controller(&self, agent_id: &str) -> Option<Arc<AgentController>> {
        self.agents.get(agent_id).map(|entry| entry.value().clone())
    }

    pub fn register_agent_controller(
        &self,
        agent_id: String,
        agent: Arc<AgentController>,
    ) -> Result<()> {
        if self.agents.insert(agent_id, agent).is_none() {
            self.update_tx.send_replace(());

            Ok(())
        } else {
            Err(anyhow!("AgentController already registered"))
        }
    }

    #[must_use]
    pub fn remove_agent_controller(&self, agent_id: &str) -> bool {
        if self.agents.remove(agent_id).is_some() {
            self.update_tx.send_replace(());

            true
        } else {
            false
        }
    }

    pub fn signal_update(&self) {
        self.update_tx.send_replace(());
    }

    #[must_use]
    pub fn total_slots(&self) -> AgentControllerPoolTotalSlots {
        let mut slots_processing = 0;
        let mut slots_total = 0;

        for entry in &self.agents {
            let agent = entry.value();

            slots_processing += agent.slots_processing.get();
            slots_total += agent.slots_total.get();
        }

        AgentControllerPoolTotalSlots {
            slots_processing,
            slots_total,
        }
    }
}

impl Default for AgentControllerPool {
    fn default() -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            agents: DashMap::new(),
            update_tx,
        }
    }
}

impl SubscribesToUpdates for AgentControllerPool {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

impl ProducesSnapshot for AgentControllerPool {
    type Snapshot = AgentControllerPoolSnapshot;

    fn make_snapshot(&self) -> Result<Self::Snapshot> {
        let mut agents: Vec<AgentControllerSnapshot> = Vec::with_capacity(self.agents.len());

        for entry in &self.agents {
            let agent_controller = entry.value();

            agents.push(agent_controller.make_snapshot()?);
        }

        Ok(AgentControllerPoolSnapshot { agents })
    }
}

#[async_trait]
impl SetsDesiredState for AgentControllerPool {
    async fn set_desired_state(&self, desired_state: AgentDesiredState) -> Result<()> {
        for agent in &self.agents {
            let agent_controller = agent.value();

            agent_controller
                .set_desired_state(desired_state.clone())
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    use tokio::sync::mpsc;
    use tokio::sync::watch;
    use tokio::time::timeout;
    use tokio_util::sync::CancellationToken;

    use super::AgentControllerPool;
    use crate::agent_controller::AgentController;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::atomic_value::AtomicValue;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;

    fn agent_controller_with_slots(
        slots_processing: i32,
        slots_total: i32,
    ) -> Arc<AgentController> {
        let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();

        Arc::new(AgentController {
            agent_message_tx,
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            connection_close: CancellationToken::new(),
            desired_slots_total: AtomicValue::<AtomicI32>::new(0),
            download_current: AtomicValue::<AtomicU64>::new(0),
            download_filename: RwLock::new(None),
            download_indeterminate: AtomicValue::<AtomicBool>::new(true),
            download_total: AtomicValue::<AtomicU64>::new(0),
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            id: "agent-test".to_owned(),
            issues: RwLock::new(BTreeSet::new()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
            model_path: RwLock::new(None),
            name: None,
            newest_update_version: AtomicValue::<AtomicI32>::new(0),
            slots_processing: AtomicValue::<AtomicI32>::new(slots_processing),
            slots_total: AtomicValue::<AtomicI32>::new(slots_total),
            state_application_status_code: AtomicValue::<AtomicI32>::new(
                AgentStateApplicationStatus::Fresh as i32,
            ),
            uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
        })
    }

    #[tokio::test]
    async fn watch_receiver_observes_send_fired_before_changed_await() {
        let (update_tx, mut update_rx) = watch::channel(());

        update_tx.send_replace(());

        assert!(
            timeout(Duration::from_secs(1), update_rx.changed())
                .await
                .is_ok(),
            "watch::Receiver must observe a send fired before .changed() is awaited"
        );
    }

    #[test]
    fn register_agent_controller_rejects_duplicate_id() {
        let pool = AgentControllerPool::default();

        assert!(
            pool.register_agent_controller(
                "duplicate".to_owned(),
                agent_controller_with_slots(0, 1),
            )
            .is_ok()
        );

        let duplicate_result = pool
            .register_agent_controller("duplicate".to_owned(), agent_controller_with_slots(0, 1));

        assert_eq!(
            duplicate_result.err().unwrap().to_string(),
            "AgentController already registered"
        );
    }

    #[test]
    fn remove_agent_controller_returns_false_for_unknown_id() {
        let pool = AgentControllerPool::default();

        assert!(!pool.remove_agent_controller("never-registered"));
    }

    #[test]
    fn total_slots_sums_processing_and_total_across_agents() {
        let pool = AgentControllerPool::default();

        pool.register_agent_controller("first".to_owned(), agent_controller_with_slots(1, 4))
            .unwrap();
        pool.register_agent_controller("second".to_owned(), agent_controller_with_slots(2, 8))
            .unwrap();

        let total_slots = pool.total_slots();

        assert_eq!(total_slots.slots_processing, 3);
        assert_eq!(total_slots.slots_total, 12);
    }

    #[test]
    fn make_snapshot_includes_each_registered_agent() {
        let pool = AgentControllerPool::default();

        pool.register_agent_controller("only".to_owned(), agent_controller_with_slots(2, 5))
            .unwrap();

        let snapshot = pool.make_snapshot().unwrap();

        assert_eq!(snapshot.agents.len(), 1);
        assert_eq!(snapshot.agents[0].slots_processing, 2);
        assert_eq!(snapshot.agents[0].slots_total, 5);
    }
}
