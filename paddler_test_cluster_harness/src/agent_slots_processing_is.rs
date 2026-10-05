use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;

pub fn agent_slots_processing_is(
    agent_id: &str,
    expected_slots_processing: u64,
) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
    let agent_id = agent_id.to_owned();

    move |snapshot| {
        snapshot.agents.iter().any(|agent| {
            agent.id == agent_id && agent.slots_processing == expected_slots_processing
        })
    }
}
