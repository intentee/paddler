use anyhow::Error;

use crate::dispatched_agent::DispatchedAgent;

pub enum BufferedRequestAgentWaitResult {
    BufferOverflow,
    Found(DispatchedAgent),
    Timeout(Error),
}
