use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;

pub fn agent_count_is(expected_count: usize) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
    move |AgentControllerPoolSnapshot { agents }| agents.len() == expected_count
}
