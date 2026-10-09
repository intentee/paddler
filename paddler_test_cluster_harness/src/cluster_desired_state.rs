use paddler_messaging::balancer_desired_state::BalancerDesiredState;

pub enum ClusterDesiredState {
    Apply(Box<BalancerDesiredState>),
    KeepStored,
}
