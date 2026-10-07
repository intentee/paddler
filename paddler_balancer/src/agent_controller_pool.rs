use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use log::debug;
use tokio::sync::watch;

use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

use super::agent_controller::AgentController;
use super::agent_controller_pool_total_slots::AgentControllerPoolTotalSlots;
use crate::agent_controller_pool_updates::AgentControllerPoolUpdates;
use crate::agent_controller_registration::AgentControllerRegistration;
use crate::agent_controller_slot_guard::AgentControllerSlotGuard;
use crate::desired_state_delivery::DesiredStateDelivery;
use crate::dispatch_candidate::DispatchCandidate;
use crate::dispatched_agent::DispatchedAgent;
use crate::registered_agent_controller_guard::RegisteredAgentControllerGuard;

pub struct AgentControllerPool {
    pub agents: DashMap<String, Arc<AgentController>>,
    inference_mode: InferenceMode,
    updates: Arc<AgentControllerPoolUpdates>,
}

impl AgentControllerPool {
    #[must_use]
    pub fn new(inference_mode: InferenceMode) -> Self {
        Self {
            agents: DashMap::new(),
            inference_mode,
            updates: Arc::new(AgentControllerPoolUpdates::default()),
        }
    }

    #[must_use]
    pub fn select_least_busy_with_capacity(&self) -> Option<DispatchCandidate> {
        let mut best: Option<DispatchCandidate> = None;

        for entry in &self.agents {
            let agent_controller = entry.value().clone();
            let snapshot = agent_controller.slots_processing.get();
            let runtime = agent_controller.reported_status.read().status.runtime;

            if !runtime.serves(self.inference_mode) || snapshot >= runtime.slots_total() {
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
            self.updates.signal();

            let slot_guard = AgentControllerSlotGuard::new(
                candidate.agent_controller.clone(),
                self.updates.clone(),
            );

            Ok(DispatchedAgent::new(candidate.agent_controller, slot_guard))
        } else {
            Err(candidate)
        }
    }

    pub async fn next_available_agent(&self) -> DispatchedAgent {
        loop {
            let agent_availability_changed = self.updates.agent_availability_changed();

            if let Some(dispatched_agent) = self.take_least_busy_agent_controller() {
                return dispatched_agent;
            }

            agent_availability_changed.await;
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
        self: &Arc<Self>,
        agent_controller: Arc<AgentController>,
    ) -> AgentControllerRegistration {
        match self.agents.entry(agent_controller.id.clone()) {
            Entry::Occupied(_registered_agent) => AgentControllerRegistration::DuplicateAgentId,
            Entry::Vacant(vacant_agent_slot) => {
                vacant_agent_slot.insert(agent_controller.clone());
                self.updates.signal();

                AgentControllerRegistration::Registered(RegisteredAgentControllerGuard {
                    agent_controller,
                    agent_controller_pool: self.clone(),
                })
            }
        }
    }

    pub fn remove_agent_controller(&self, agent_id: &str) {
        self.agents.remove(agent_id);
        self.updates.signal();
    }

    pub fn set_desired_state(&self, desired_state: &AgentDesiredState) {
        for agent in &self.agents {
            let agent_controller = agent.value();

            if matches!(
                agent_controller.set_desired_state(desired_state.clone()),
                DesiredStateDelivery::AgentDisconnected
            ) {
                let agent_id = &agent_controller.id;

                debug!("Skipping the desired state for disconnected agent {agent_id}");
            }
        }
    }

    pub fn signal_update(&self) {
        self.updates.signal();
    }

    #[must_use]
    pub fn total_slots(&self) -> AgentControllerPoolTotalSlots {
        let mut slots_processing = 0;
        let mut slots_total = 0;

        for entry in &self.agents {
            let agent = entry.value();

            slots_processing += agent.slots_processing.get();
            slots_total += agent.reported_status.read().status.runtime.slots_total();
        }

        AgentControllerPoolTotalSlots {
            slots_processing,
            slots_total,
        }
    }
}

impl SubscribesToUpdates for AgentControllerPool {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.updates.subscribe()
    }
}

impl ProducesSnapshot for AgentControllerPool {
    type Snapshot = AgentControllerPoolSnapshot;

    fn make_snapshot(&self) -> Self::Snapshot {
        AgentControllerPoolSnapshot {
            agents: self
                .agents
                .iter()
                .map(|entry| entry.value().make_snapshot())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;

    use parking_lot::RwLock;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
    use paddler_messaging::agent_status::AgentStatus;
    use paddler_messaging::atomic_value::AtomicValue;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
    use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::AgentControllerPool;
    use crate::agent_controller::AgentController;
    use crate::agent_controller_registration::AgentControllerRegistration;
    use crate::agent_response_senders::AgentResponseSenders;

    struct AgentWithInbox {
        controller: Arc<AgentController>,
        inbox: mpsc::UnboundedReceiver<AgentJsonRpcMessage>,
    }

    fn agent_with_inbox(agent_id: &str) -> AgentWithInbox {
        agent_reporting(agent_id, AgentRuntimeStatus::Idle)
    }

    fn agent_reporting(agent_id: &str, runtime: AgentRuntimeStatus) -> AgentWithInbox {
        let (agent_message_tx, inbox) = mpsc::unbounded_channel();

        AgentWithInbox {
            controller: Arc::new(AgentController {
                agent_message_tx,
                agent_response_senders: AgentResponseSenders::default(),
                connection_close: CancellationToken::new(),
                id: agent_id.to_owned(),
                name: None,
                reported_status: RwLock::new(SlotAggregatedStatusSnapshot {
                    status: AgentStatus {
                        runtime,
                        ..AgentStatus::default()
                    },
                    version: 0,
                }),
                slots_processing: AtomicValue::<AtomicU64>::new(0),
            }),
            inbox,
        }
    }

    fn serving(inference_mode: InferenceMode) -> AgentRuntimeStatus {
        AgentRuntimeStatus::Serving {
            inference_mode,
            slots_total: 1,
        }
    }

    #[test]
    fn dispatches_only_to_agents_serving_the_cluster_mode() {
        let pool = Arc::new(AgentControllerPool::new(InferenceMode::Embeddings));
        let registrations = [
            pool.register_agent_controller(
                agent_reporting("text-generation", serving(InferenceMode::TextGeneration))
                    .controller,
            ),
            pool.register_agent_controller(
                agent_reporting("embeddings", serving(InferenceMode::Embeddings)).controller,
            ),
        ];

        assert!(registrations.iter().all(|registration| matches!(
            registration,
            AgentControllerRegistration::Registered(_)
        )));
        assert_eq!(
            pool.select_least_busy_with_capacity()
                .map(|candidate| candidate.agent_controller.id.clone()),
            Some("embeddings".to_owned())
        );
    }

    #[test]
    fn counts_only_the_slots_of_serving_agents() {
        let pool = Arc::new(AgentControllerPool::new(InferenceMode::TextGeneration));
        let _registrations = [
            pool.register_agent_controller(agent_with_inbox("idle").controller),
            pool.register_agent_controller(
                agent_reporting("serving", serving(InferenceMode::TextGeneration)).controller,
            ),
        ];

        assert_eq!(pool.total_slots().slots_total, 1);
    }

    #[test]
    fn delivers_the_desired_state_to_connected_agents_past_a_disconnected_one() {
        let pool = Arc::new(AgentControllerPool::new(InferenceMode::TextGeneration));
        let disconnected_agent = agent_with_inbox("disconnected");
        let mut connected_agent = agent_with_inbox("connected");

        drop(disconnected_agent.inbox);

        let registrations = [
            pool.register_agent_controller(disconnected_agent.controller),
            pool.register_agent_controller(connected_agent.controller),
        ];

        assert!(registrations.iter().all(|registration| matches!(
            registration,
            AgentControllerRegistration::Registered(_)
        )));

        pool.set_desired_state(&AgentDesiredState::from(
            BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
        ));

        assert!(matches!(
            connected_agent.inbox.try_recv(),
            Ok(AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::SetState(set_state_params)
            )) if set_state_params.desired_state == AgentDesiredState::from(BalancerDesiredState::unconfigured(InferenceMode::TextGeneration))
        ));
    }
}
