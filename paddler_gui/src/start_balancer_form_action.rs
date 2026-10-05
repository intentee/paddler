use paddler_bootstrap::balancer_bootstrap_config::BalancerBootstrapConfig;

pub enum StartBalancerFormAction {
    None,
    Cancel,
    StartBalancer(Box<BalancerBootstrapConfig>),
}
