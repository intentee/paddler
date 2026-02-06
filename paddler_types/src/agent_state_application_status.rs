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
