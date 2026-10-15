use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[derive(Clone, Debug, PartialEq)]
pub enum StoredBalancerDesiredState {
    NothingStored,
    Stored(BalancerDesiredState),
}
