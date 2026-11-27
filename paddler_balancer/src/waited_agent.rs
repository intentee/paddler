use crate::cluster_lacks_capability::ClusterLacksCapability;
use crate::dispatched_agent::DispatchedAgent;

pub enum WaitedAgent {
    CapabilityLacking(ClusterLacksCapability),
    Dispatched(DispatchedAgent),
}
