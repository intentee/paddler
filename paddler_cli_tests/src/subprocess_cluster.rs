use paddler_test_cluster_harness::cluster::Cluster;

use crate::subprocess_signals::SubprocessSignals;

pub struct SubprocessCluster {
    pub balancer_signals: SubprocessSignals,
    pub cluster: Cluster,
}
