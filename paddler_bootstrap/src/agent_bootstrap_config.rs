use std::net::SocketAddr;

use crate::agent_runtime_config::AgentRuntimeConfig;

pub struct AgentBootstrapConfig {
    pub management_address: SocketAddr,
    pub runtime: AgentRuntimeConfig,
}
