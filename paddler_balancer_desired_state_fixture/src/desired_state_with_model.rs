use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[must_use]
pub fn desired_state_with_model(model_uri: &str) -> BalancerDesiredState {
    BalancerDesiredState {
        model: AgentDesiredModel::Uri(model_uri.to_owned()),
        ..BalancerDesiredState::default()
    }
}
