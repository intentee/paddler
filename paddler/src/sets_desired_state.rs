use anyhow::Result;
use async_trait::async_trait;
use paddler_types::agent_desired_state::AgentDesiredState;

#[async_trait]
pub trait SetsDesiredState {
    async fn set_desired_state(&self, desired_state: AgentDesiredState) -> Result<()>;
}
