use std::sync::Arc;

use anyhow::Error;

use crate::balancer::agent_controller::AgentController;

pub enum BufferedRequestAgentWaitResult {
    BufferOverflow,
    Found(Arc<AgentController>),
    Timeout(Error),
}
