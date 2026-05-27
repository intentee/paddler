use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;

pub fn assert_slots_total_at_least(
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
