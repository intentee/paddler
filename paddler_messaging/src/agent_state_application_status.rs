use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum AgentStateApplicationStatus {
    Applied,
    AttemptedAndNotAppliable,
    AttemptedAndRetrying,
    #[default]
    Fresh,
    Stuck,
}

impl AgentStateApplicationStatus {
    #[must_use]
    pub const fn should_try_to_apply(&self) -> bool {
        match self {
            Self::Applied | Self::AttemptedAndNotAppliable => false,
            Self::AttemptedAndRetrying | Self::Fresh | Self::Stuck => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentStateApplicationStatus;

    #[test]
    fn applied_should_not_try_to_apply() {
        assert!(!AgentStateApplicationStatus::Applied.should_try_to_apply());
    }

    #[test]
    fn attempted_and_not_appliable_should_not_try_to_apply() {
        assert!(!AgentStateApplicationStatus::AttemptedAndNotAppliable.should_try_to_apply());
    }

    #[test]
    fn attempted_and_retrying_should_try_to_apply() {
        assert!(AgentStateApplicationStatus::AttemptedAndRetrying.should_try_to_apply());
    }

    #[test]
    fn fresh_should_try_to_apply() {
        assert!(AgentStateApplicationStatus::Fresh.should_try_to_apply());
    }

    #[test]
    fn stuck_should_try_to_apply() {
        assert!(AgentStateApplicationStatus::Stuck.should_try_to_apply());
    }
}
