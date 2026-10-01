use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use log::debug;
use tokio::sync::watch;

use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

use super::agent_controller::AgentController;
use super::agent_controller_pool_total_slots::AgentControllerPoolTotalSlots;
use crate::agent_controller_registration::AgentControllerRegistration;
use crate::agent_controller_slot_guard::AgentControllerSlotGuard;
use crate::desired_state_delivery::DesiredStateDelivery;
use crate::dispatch_candidate::DispatchCandidate;
use crate::dispatched_agent::DispatchedAgent;
use crate::registered_agent_controller_guard::RegisteredAgentControllerGuard;

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

            if snapshot >= agent_controller.reported_status.read().status.slots_total {
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
        self: &Arc<Self>,
        agent_controller: Arc<AgentController>,
    ) -> AgentControllerRegistration {
        match self.agents.entry(agent_controller.id.clone()) {
            Entry::Occupied(_registered_agent) => AgentControllerRegistration::DuplicateAgentId,
            Entry::Vacant(vacant_agent_slot) => {
                vacant_agent_slot.insert(agent_controller.clone());
                self.update_tx.send_replace(());

                AgentControllerRegistration::Registered(RegisteredAgentControllerGuard {
                    agent_controller,
                    agent_controller_pool: self.clone(),
                })
            }
        }
    }

    pub fn remove_agent_controller(&self, agent_id: &str) {
        self.agents.remove(agent_id);
        self.update_tx.send_replace(());
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
        self.update_tx.send_replace(());
    }

    #[must_use]
    pub fn total_slots(&self) -> AgentControllerPoolTotalSlots {
        let mut slots_processing = 0;
        let mut slots_total = 0;

        for entry in &self.agents {
            let agent = entry.value();

            slots_processing += agent.slots_processing.get();
            slots_total += agent.reported_status.read().status.slots_total;
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
    use paddler_messaging::atomic_value::AtomicValue;
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
        let (agent_message_tx, inbox) = mpsc::unbounded_channel();

        AgentWithInbox {
            controller: Arc::new(AgentController {
                agent_message_tx,
                agent_response_senders: AgentResponseSenders::default(),
                connection_close: CancellationToken::new(),
                id: agent_id.to_owned(),
                name: None,
                reported_status: RwLock::new(SlotAggregatedStatusSnapshot::default()),
                slots_processing: AtomicValue::<AtomicU64>::new(0),
            }),
            inbox,
        }
    }

    #[test]
    fn delivers_the_desired_state_to_connected_agents_past_a_disconnected_one() {
        let pool = Arc::new(AgentControllerPool::default());
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

        pool.set_desired_state(&AgentDesiredState::default());

        assert!(matches!(
            connected_agent.inbox.try_recv(),
            Ok(AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::SetState(set_state_params)
            )) if set_state_params.desired_state == AgentDesiredState::default()
        ));
    }
}
