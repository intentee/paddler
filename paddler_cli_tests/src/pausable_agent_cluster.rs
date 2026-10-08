use paddler_test_cluster_harness::cluster::Cluster;

use crate::pausable_agent::PausableAgent;

pub struct PausableAgentCluster {
    pub cluster: Cluster,
    pub pausable_agent: PausableAgent,
}
