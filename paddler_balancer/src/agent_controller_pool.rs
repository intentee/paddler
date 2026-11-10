use std::collections::BTreeSet;
use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use tokio::sync::futures::Notified;
use tokio::sync::watch;

use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

use super::agent_controller::AgentController;
use super::agent_controller_pool_total_slots::AgentControllerPoolTotalSlots;
use crate::agent_controller_pool_updates::AgentControllerPoolUpdates;
use crate::agent_controller_slot_guard::AgentControllerSlotGuard;
use crate::balancer_applicable_state::BalancerApplicableState;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_counter::BufferedRequestCounter;
use crate::dispatch_candidate::DispatchCandidate;
use crate::dispatch_position::DispatchPosition;
use crate::dispatch_requirement::DispatchRequirement;
use crate::dispatch_waiting_line::DispatchWaitingLine;
use crate::dispatched_agent::DispatchedAgent;
use crate::waiting_line_place::WaitingLinePlace;

pub struct AgentControllerPool {
    agents: DashMap<String, Arc<AgentController>>,
    dispatch_waiting_line: DispatchWaitingLine,
    updates: Arc<AgentControllerPoolUpdates>,
}

impl AgentControllerPool {
    pub fn agent_availability_changed(&self) -> Notified<'_> {
        self.updates.agent_availability_changed()
    }

    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    #[must_use]
    pub fn get_agent_controller(&self, agent_id: &str) -> Option<Arc<AgentController>> {
        self.agents.get(agent_id).map(|entry| entry.value().clone())
    }

    #[must_use]
    pub fn join_waiting_line_when_admitted(
        &self,
        dispatch_requirement: DispatchRequirement,
        buffered_request_counter: &Arc<BufferedRequestCounter>,
    ) -> Option<WaitingLinePlace<'_>> {
        WaitingLinePlace::join_when_admitted(
            &self.dispatch_waiting_line,
            dispatch_requirement,
            buffered_request_counter,
        )
    }

    #[must_use]
    pub fn model_paths_serving(
        &self,
        inference_capability: InferenceCapability,
    ) -> BTreeSet<String> {
        self.agents
            .iter()
            .filter_map(|agent| {
                let reported_status = agent.value().reported_status.read();

                reported_status
                    .status
                    .runtime
                    .serves(inference_capability)
                    .then(|| reported_status.status.model_path.clone())
                    .flatten()
            })
            .collect()
    }

    pub async fn next_available_agent<'pool>(
        &'pool self,
        mut agent_availability_changed: Notified<'pool>,
        dispatch_requirement: DispatchRequirement,
        dispatch_position: DispatchPosition,
    ) -> DispatchedAgent {
        loop {
            agent_availability_changed.await;
            agent_availability_changed = self.agent_availability_changed();

            if let Some(dispatched_agent) =
                self.take_least_busy_agent_controller(dispatch_requirement, dispatch_position)
            {
                return dispatched_agent;
            }
        }
    }

    pub fn remove_agent_controller(&self, agent_id: &str) {
        self.agents.remove(agent_id);
        self.updates.signal();
    }

    #[must_use]
    pub fn select_least_busy_with_capacity(
        &self,
        dispatch_requirement: DispatchRequirement,
        dispatch_position: DispatchPosition,
    ) -> Option<DispatchCandidate> {
        let requirements_waiting_ahead = self
            .dispatch_waiting_line
            .requirements_ahead_of(dispatch_position);
        let mut best: Option<DispatchCandidate> = None;

        for entry in &self.agents {
            let agent_controller = entry.value().clone();
            let snapshot = agent_controller.slots_processing.get();
            let agent_runtime_status = agent_controller.reported_status.read().status.runtime;
            let Some(reserved_slots) = snapshot
                .checked_add(requirements_waiting_ahead.slots_held_on(&agent_runtime_status))
                .and_then(|occupied_slots| {
                    dispatch_requirement.reservable_slots(&agent_runtime_status, occupied_slots)
                })
            else {
                continue;
            };

            best = Some(match best {
                Some(current) if current.snapshot <= snapshot => current,
                _ => DispatchCandidate {
                    agent_controller,
                    reserved_slots,
                    snapshot,
                },
            });
        }

        best
    }

    pub fn set_desired_state(&self, balancer_applicable_state: &BalancerApplicableState) {
        for agent in &self.agents {
            agent.value().set_desired_state(balancer_applicable_state);
        }
    }

    pub fn signal_update(&self) {
        self.updates.signal();
    }

    #[must_use]
    pub fn take_least_busy_agent_controller(
        &self,
        dispatch_requirement: DispatchRequirement,
        dispatch_position: DispatchPosition,
    ) -> Option<DispatchedAgent> {
        loop {
            let candidate =
                self.select_least_busy_with_capacity(dispatch_requirement, dispatch_position)?;

            if let Ok(dispatched) = self.try_claim(candidate) {
                return Some(dispatched);
            }
        }
    }

    #[must_use]
    pub fn total_slots(&self) -> AgentControllerPoolTotalSlots {
        AgentControllerPoolTotalSlots {
            slots_processing: self
                .agents
                .iter()
                .map(|entry| entry.value().slots_processing.get())
                .sum(),
            slots_total: self
                .agents
                .iter()
                .map(|entry| {
                    entry
                        .value()
                        .reported_status
                        .read()
                        .status
                        .runtime
                        .slots_total()
                })
                .sum(),
        }
    }

    /// # Errors
    ///
    /// Returns the [`DispatchCandidate`] back when another dispatch changed the slots its agent is processing since the candidate was selected, or when the slots it would claim exceed [`u64::MAX`].
    pub fn try_claim(
        &self,
        candidate: DispatchCandidate,
    ) -> Result<DispatchedAgent, DispatchCandidate> {
        if let Some(claimed_slots) = candidate
            .snapshot
            .checked_add(u64::from(candidate.reserved_slots.get()))
            && candidate
                .agent_controller
                .slots_processing
                .compare_and_swap(candidate.snapshot, claimed_slots)
        {
            self.updates.signal();

            let slot_guard = AgentControllerSlotGuard::new(
                candidate.agent_controller.clone(),
                self.updates.clone(),
                candidate.reserved_slots,
            );

            Ok(DispatchedAgent::new(candidate.agent_controller, slot_guard))
        } else {
            Err(candidate)
        }
    }

    #[must_use]
    pub fn try_insert_agent_controller(
        &self,
        agent_controller: &Arc<AgentController>,
        balancer_applicable_state_holder: &BalancerApplicableStateHolder,
    ) -> bool {
        match self.agents.entry(agent_controller.id.clone()) {
            Entry::Occupied(_registered_agent) => false,
            Entry::Vacant(vacant_agent_slot) => {
                vacant_agent_slot
                    .insert(agent_controller.clone())
                    .set_desired_state(
                        &balancer_applicable_state_holder.get_balancer_applicable_state(),
                    );
                self.updates.signal();

                true
            }
        }
    }
}

impl Default for AgentControllerPool {
    fn default() -> Self {
        let updates = Arc::new(AgentControllerPoolUpdates::default());

        Self {
            agents: DashMap::new(),
            dispatch_waiting_line: DispatchWaitingLine::new(updates.clone()),
            updates,
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
