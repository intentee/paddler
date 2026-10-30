use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum AgentStateApplicationStatus {
    Applied,
    AttemptedAndRetrying,
    #[default]
    Fresh,
    Stuck,
}
