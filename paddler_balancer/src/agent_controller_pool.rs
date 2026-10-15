use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use tokio::sync::futures::Notified;
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
use crate::dispatch_candidate::DispatchCandidate;
use crate::dispatched_agent::DispatchedAgent;
use crate::registered_agent_controller_guard::RegisteredAgentControllerGuard;

#[derive(Default)]
pub struct AgentControllerPool {
    agents: DashMap<String, Arc<AgentController>>,
    updates: Arc<AgentControllerPoolUpdates>,
}

impl AgentControllerPool {
    #[must_use]
    pub fn select_least_busy_with_capacity(
        &self,
        inference_mode: InferenceMode,
    ) -> Option<DispatchCandidate> {
        let mut best: Option<DispatchCandidate> = None;

        for entry in &self.agents {
            let agent_controller = entry.value().clone();
            let snapshot = agent_controller.slots_processing.get();
            let slots_serving = agent_controller
                .reported_status
                .read()
                .status
                .runtime
                .slots_serving(inference_mode);

            if snapshot >= slots_serving {
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

    pub fn agent_availability_changed(&self) -> Notified<'_> {
        self.updates.agent_availability_changed()
    }

    pub async fn next_available_agent<'pool>(
        &'pool self,
        mut agent_availability_changed: Notified<'pool>,
        inference_mode: InferenceMode,
    ) -> DispatchedAgent {
        loop {
            agent_availability_changed.await;
            agent_availability_changed = self.agent_availability_changed();

            if let Some(dispatched_agent) = self.take_least_busy_agent_controller(inference_mode) {
                return dispatched_agent;
            }
        }
    }

    #[must_use]
    pub fn take_least_busy_agent_controller(
        &self,
        inference_mode: InferenceMode,
    ) -> Option<DispatchedAgent> {
        loop {
            let candidate = self.select_least_busy_with_capacity(inference_mode)?;

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
            agent.value().set_desired_state(desired_state.clone());
        }
    }

    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.agents.len()
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

    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
    use paddler_messaging::agent_status::AgentStatus;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
    use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::AgentControllerPool;
    use crate::agent_controller_fixture::AgentControllerFixture;
    use crate::agent_controller_registration::AgentControllerRegistration;

    fn agent_with_inbox(agent_id: &str) -> AgentControllerFixture {
        agent_reporting(agent_id, AgentRuntimeStatus::Idle)
    }

    fn agent_reporting(agent_id: &str, runtime: AgentRuntimeStatus) -> AgentControllerFixture {
        AgentControllerFixture::reporting(
            agent_id,
            SlotAggregatedStatusSnapshot {
                status: AgentStatus {
                    runtime,
                    ..AgentStatus::default()
                },
                version: 0,
            },
        )
    }

    fn serving(inference_mode: InferenceMode) -> AgentRuntimeStatus {
        AgentRuntimeStatus::Serving {
            inference_mode,
            slots_total: 1,
        }
    }

    #[test]
    fn dispatches_only_to_agents_with_free_slots() {
        let pool = Arc::new(AgentControllerPool::default());
        let registrations = [
            pool.register_agent_controller(agent_with_inbox("idle").agent_controller),
            pool.register_agent_controller(
                agent_reporting("embeddings", serving(InferenceMode::Embeddings)).agent_controller,
            ),
        ];

        assert_eq!(
            registrations
                .iter()
                .map(AgentControllerRegistration::registered_agent_id)
                .collect::<Vec<_>>(),
            vec![Some("idle"), Some("embeddings")]
        );
        assert_eq!(
            pool.select_least_busy_with_capacity(InferenceMode::Embeddings)
                .map(|candidate| candidate.agent_controller.id.clone()),
            Some("embeddings".to_owned())
        );
    }

    #[test]
    fn refuses_a_second_agent_with_the_same_id() {
        let pool = Arc::new(AgentControllerPool::default());
        let registrations = [
            pool.register_agent_controller(agent_with_inbox("twin").agent_controller),
            pool.register_agent_controller(agent_with_inbox("twin").agent_controller),
        ];

        assert_eq!(
            registrations
                .iter()
                .map(AgentControllerRegistration::registered_agent_id)
                .collect::<Vec<_>>(),
            vec![Some("twin"), None]
        );
    }

    #[test]
    fn does_not_dispatch_to_agents_serving_another_inference_mode() {
        let pool = Arc::new(AgentControllerPool::default());
        let _registration = pool.register_agent_controller(
            agent_reporting("decision", serving(InferenceMode::Decision)).agent_controller,
        );

        assert_eq!(
            [InferenceMode::Decision, InferenceMode::TextGeneration].map(|inference_mode| {
                pool.select_least_busy_with_capacity(inference_mode)
                    .map(|candidate| candidate.agent_controller.id.clone())
            }),
            [Some("decision".to_owned()), None]
        );
    }

    #[test]
    fn counts_only_the_slots_of_serving_agents() {
        let pool = Arc::new(AgentControllerPool::default());
        let _registrations = [
            pool.register_agent_controller(agent_with_inbox("idle").agent_controller),
            pool.register_agent_controller(
                agent_reporting("serving", serving(InferenceMode::TextGeneration)).agent_controller,
            ),
        ];

        assert_eq!(pool.total_slots().slots_total, 1);
    }

    #[test]
    fn delivers_the_desired_state_to_connected_agents_past_a_disconnected_one() {
        let pool = Arc::new(AgentControllerPool::default());
        let disconnected_agent = agent_with_inbox("disconnected");
        let mut connected_agent = agent_with_inbox("connected");

        drop(disconnected_agent.agent_message_rx);

        let registrations = [
            pool.register_agent_controller(disconnected_agent.agent_controller),
            pool.register_agent_controller(connected_agent.agent_controller),
        ];

        assert!(registrations.iter().all(|registration| matches!(
            registration,
            AgentControllerRegistration::Registered(_)
        )));

        pool.set_desired_state(&AgentDesiredState::from(BalancerDesiredState::default()));

        assert!(matches!(
            connected_agent.agent_message_rx.try_recv(),
            Ok(AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::SetState(set_state_params)
            )) if set_state_params.desired_state == AgentDesiredState::from(BalancerDesiredState::default())
        ));
    }
}
