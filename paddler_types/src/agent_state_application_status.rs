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
    #[must_use]
    pub const fn should_try_to_apply(&self) -> bool {
        match self {
            Self::Applied | Self::AttemptedAndNotAppliable => false,
            Self::AttemptedAndRetrying | Self::Fresh | Self::Stuck => true,
        }
    }
}

impl TryFrom<i32> for AgentStateApplicationStatus {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Applied),
            1 => Ok(Self::AttemptedAndNotAppliable),
            2 => Ok(Self::AttemptedAndRetrying),
            3 => Ok(Self::Fresh),
            4 => Ok(Self::Stuck),
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
    fn try_from_valid_values() -> Result<()> {
        assert_eq!(
            AgentStateApplicationStatus::try_from(0)?,
            AgentStateApplicationStatus::Applied
        );
        assert_eq!(
            AgentStateApplicationStatus::try_from(1)?,
            AgentStateApplicationStatus::AttemptedAndNotAppliable
        );
        assert_eq!(
            AgentStateApplicationStatus::try_from(2)?,
            AgentStateApplicationStatus::AttemptedAndRetrying
        );
        assert_eq!(
            AgentStateApplicationStatus::try_from(3)?,
            AgentStateApplicationStatus::Fresh
        );
        assert_eq!(
            AgentStateApplicationStatus::try_from(4)?,
            AgentStateApplicationStatus::Stuck
        );

        Ok(())
    }

    #[test]
    fn try_from_invalid_value_fails() {
        assert!(AgentStateApplicationStatus::try_from(5).is_err());
    }
}
