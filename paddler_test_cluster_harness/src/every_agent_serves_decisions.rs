use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;

use crate::agent_serves_decisions::agent_serves_decisions;

pub fn every_agent_serves_decisions(
    agent_ids: &[String],
    decisions: AgentDecisionsServing,
) -> impl Fn(&AgentControllerPoolSnapshot) -> bool + use<'_> {
    move |AgentControllerPoolSnapshot { agents }| {
        agent_ids.iter().all(|agent_id| {
            agents
                .iter()
                .any(|agent| agent.id == *agent_id && agent_serves_decisions(agent, decisions))
        })
    }
}
