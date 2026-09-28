use tokio_util::sync::CancellationToken;

pub struct AgentRunnerParams {
    pub agent_name: Option<String>,
    pub cancellation_token: CancellationToken,
    pub management_address: String,
    pub slots: i32,
}
