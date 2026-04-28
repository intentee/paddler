use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;

pub struct AgentsStatus;

impl AgentsStatus {
    pub fn agent_count_is(expected_count: usize) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        move |snapshot| snapshot.agents.len() == expected_count
    }

    pub fn slots_processing_is(
        agent_id: &str,
        expected_slots_processing: i32,
    ) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        let agent_id = agent_id.to_owned();

        move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == agent_id && agent.slots_processing == expected_slots_processing
            })
        }
    }

    pub fn slots_total_at_least(
        agent_id: &str,
        expected_slots_total: i32,
    ) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        let agent_id = agent_id.to_owned();

        move |snapshot| {
            snapshot
                .agents
                .iter()
                .any(|agent| agent.id == agent_id && agent.slots_total >= expected_slots_total)
        }
    }
}
