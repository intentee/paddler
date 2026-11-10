use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;

pub fn agent_slots_processing_is(
    agent_id: &str,
    expected_slots_processing: u64,
) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
    let agent_id = agent_id.to_owned();

    move |AgentControllerPoolSnapshot { agents }| {
        agents.iter().any(
            |AgentControllerSnapshot {
                 id,
                 slots_processing,
                 ..
             }| { *id == agent_id && *slots_processing == expected_slots_processing },
        )
    }
}
