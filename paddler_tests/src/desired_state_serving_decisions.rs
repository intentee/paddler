use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;

const POINTER_HEAD_NO_AGENT_LOADS: &str = "agent:///pointer-head-no-agent-loads.gguf";

#[must_use]
pub fn desired_state_serving_decisions() -> BalancerDesiredState {
    BalancerDesiredState {
        decision: DecisionSettings {
            pointer_head: AgentDesiredModel::Uri(POINTER_HEAD_NO_AGENT_LOADS.to_owned()),
        },
        ..BalancerDesiredState::default()
    }
}
