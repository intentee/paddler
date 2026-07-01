use anyhow::Result;
use anyhow::anyhow;

use crate::agent_response::AgentsResponse;

#[derive(Debug, Default)]
pub struct BalancerManagementClient {}

impl BalancerManagementClient {
    pub async fn fetch_agents(&self) -> Result<AgentsResponse> {
        let response = reqwest::get("http://127.0.0.1:8095/api/v1/agents").await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get agent status: {}", response.status()));
        }

        Ok(response.json::<AgentsResponse>().await?)
    }
}
