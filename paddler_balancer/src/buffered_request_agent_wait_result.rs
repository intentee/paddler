use crate::cluster_lacks_capability::ClusterLacksCapability;
use crate::dispatched_agent::DispatchedAgent;

pub enum BufferedRequestAgentWaitResult {
    BufferOverflow,
    CapabilityLacking(ClusterLacksCapability),
    Found(DispatchedAgent),
    Timeout,
}
