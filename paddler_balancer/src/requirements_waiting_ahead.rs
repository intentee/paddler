use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;

use crate::dispatch_requirement::DispatchRequirement;

pub struct RequirementsWaitingAhead {
    pub dispatch_requirements: Vec<DispatchRequirement>,
}

impl RequirementsWaitingAhead {
    #[must_use]
    pub fn slots_held_on(&self, agent_runtime_status: &AgentRuntimeStatus) -> u64 {
        self.dispatch_requirements
            .iter()
            .map(|dispatch_requirement| {
                dispatch_requirement.slots_held_while_waiting(agent_runtime_status)
            })
            .sum()
    }
}
