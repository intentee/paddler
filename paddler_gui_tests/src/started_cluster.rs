use paddler_balancer::balancer_addresses::BalancerAddresses;

use crate::task_actions::TaskActions;

pub struct StartedCluster {
    pub addresses: BalancerAddresses,
    pub messages: TaskActions,
}
