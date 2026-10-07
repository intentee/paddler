#[derive(Debug, Eq, PartialEq)]
pub struct AgentRunnerConfig {
    pub agent_name: Option<String>,
    pub management_address: String,
    pub slots: u16,
}
