use anyhow::Error;

use crate::balancer::dispatched_agent::DispatchedAgent;

pub enum BufferedRequestAgentWaitResult {
    BufferOverflow,
    Found(DispatchedAgent),
    Timeout(Error),
}
