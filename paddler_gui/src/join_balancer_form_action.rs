use paddler_bootstrap::agent_bootstrap_config::AgentBootstrapConfig;

#[derive(Debug, Eq, PartialEq)]
pub enum JoinBalancerFormAction {
    None,
    Cancel,
    ConnectAgent(AgentBootstrapConfig),
}
