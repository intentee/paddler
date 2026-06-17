use paddler_messaging::balancer_desired_state::BalancerDesiredState;

pub enum DesiredStateInit {
    Inherit,
    Set(Box<BalancerDesiredState>),
}

impl DesiredStateInit {
    #[must_use]
    pub fn set(desired_state: BalancerDesiredState) -> Self {
        Self::Set(Box::new(desired_state))
    }
}
