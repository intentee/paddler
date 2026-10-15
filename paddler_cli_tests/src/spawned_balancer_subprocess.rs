use paddler_test_cluster_harness::running_balancer::RunningBalancer;

use crate::subprocess_signals::SubprocessSignals;

pub struct SpawnedBalancerSubprocess {
    pub running_balancer: RunningBalancer,
    pub signals: SubprocessSignals,
}
