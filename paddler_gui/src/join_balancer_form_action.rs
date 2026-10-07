use paddler_agent_runner::agent_runner_config::AgentRunnerConfig;

#[derive(Debug, Eq, PartialEq)]
pub enum JoinBalancerFormAction {
    None,
    Cancel,
    ConnectAgent(AgentRunnerConfig),
}
