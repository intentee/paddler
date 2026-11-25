use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[derive(Clone)]
pub enum ClusterDesiredState {
    Apply(Box<BalancerDesiredState>),
    KeepStored,
}
