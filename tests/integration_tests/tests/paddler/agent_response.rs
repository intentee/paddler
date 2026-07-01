use std::time::SystemTime;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct AgentStatus {
    pub agent_name: String,
    pub error: Option<String>,
    pub is_connect_error: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct Agent {
    pub last_update: SystemTime,
    pub status: AgentStatus,
}

#[derive(Deserialize, Debug)]
pub struct AgentsResponse {
    pub agents: Vec<Agent>,
}
