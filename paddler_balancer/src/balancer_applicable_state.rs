use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::desired_state_replacement_error::DesiredStateReplacementError;

#[derive(Clone, Debug)]
pub struct BalancerApplicableState {
    pub agent_desired_state: AgentDesiredState,
    pub balancer_desired_state: BalancerDesiredState,
    pub version: u64,
}

impl BalancerApplicableState {
    #[must_use]
    pub fn initial(balancer_desired_state: BalancerDesiredState) -> Self {
        Self::at_version(balancer_desired_state, 0)
    }

    fn at_version(balancer_desired_state: BalancerDesiredState, version: u64) -> Self {
        Self {
            agent_desired_state: AgentDesiredState::from(&balancer_desired_state),
            balancer_desired_state,
            version,
        }
    }

    /// # Errors
    ///
    /// Returns [`DesiredStateReplacementError::VersionsExhausted`] when this state is at the last representable version, so no version can follow it.
    pub fn superseded_by(
        &self,
        balancer_desired_state: BalancerDesiredState,
    ) -> Result<Self, DesiredStateReplacementError> {
        self.version
            .checked_add(1)
            .map(|superseding_version| {
                Self::at_version(balancer_desired_state, superseding_version)
            })
            .ok_or(DesiredStateReplacementError::VersionsExhausted {
                version: self.version,
            })
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    use super::BalancerApplicableState;
    use crate::desired_state_replacement_error::DesiredStateReplacementError;

    #[test]
    fn a_state_at_the_last_representable_version_cannot_be_superseded() {
        assert!(matches!(
            BalancerApplicableState::at_version(BalancerDesiredState::default(), u64::MAX)
                .superseded_by(BalancerDesiredState::default()),
            Err(DesiredStateReplacementError::VersionsExhausted { version }) if version == u64::MAX
        ));
    }
}
