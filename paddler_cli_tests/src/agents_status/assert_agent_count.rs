use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;

pub fn assert_agent_count(expected_count: usize) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
    move |snapshot| snapshot.agents.len() == expected_count
}
