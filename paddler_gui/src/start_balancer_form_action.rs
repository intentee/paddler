use paddler_balancer_runner::balancer_runner_config::BalancerRunnerConfig;

pub enum StartBalancerFormAction {
    None,
    Cancel,
    StartBalancer(Box<BalancerRunnerConfig>),
}
