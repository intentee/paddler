use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(i32)]
pub enum AgentStateApplicationStatus {
    Applied = 0,
    AttemptedAndNotAppliable = 1,
    AttemptedAndRetrying = 2,
    Fresh = 3,
    Stuck = 4,
}

impl AgentStateApplicationStatus {
    pub fn should_try_to_apply(&self) -> bool {
        match self {
            AgentStateApplicationStatus::Applied => false,
            AgentStateApplicationStatus::AttemptedAndNotAppliable => false,
            AgentStateApplicationStatus::AttemptedAndRetrying => true,
            AgentStateApplicationStatus::Fresh => true,
            AgentStateApplicationStatus::Stuck => true,
        }
    }
}

impl TryFrom<i32> for AgentStateApplicationStatus {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AgentStateApplicationStatus::Applied),
            1 => Ok(AgentStateApplicationStatus::AttemptedAndNotAppliable),
            2 => Ok(AgentStateApplicationStatus::AttemptedAndRetrying),
            3 => Ok(AgentStateApplicationStatus::Fresh),
            4 => Ok(AgentStateApplicationStatus::Stuck),
            _ => Err(anyhow!(
                "Invalid value for AgentStateApplicationStatus: {value}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn try_from_valid_values() {
        assert_eq!(
            AgentStateApplicationStatus::try_from(0).unwrap(),
            AgentStateApplicationStatus::Applied
        );
        assert_eq!(
            AgentStateApplicationStatus::try_from(1).unwrap(),
            AgentStateApplicationStatus::AttemptedAndNotAppliable
        );
        assert_eq!(
            AgentStateApplicationStatus::try_from(2).unwrap(),
            AgentStateApplicationStatus::AttemptedAndRetrying
        );
        assert_eq!(
            AgentStateApplicationStatus::try_from(3).unwrap(),
            AgentStateApplicationStatus::Fresh
        );
        assert_eq!(
            AgentStateApplicationStatus::try_from(4).unwrap(),
            AgentStateApplicationStatus::Stuck
        );
    }

    #[test]
    fn try_from_invalid_value_fails() {
        assert!(AgentStateApplicationStatus::try_from(5).is_err());
    }
}
