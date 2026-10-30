use std::path::PathBuf;

use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicableDecisions {
    Disabled,
    Enabled { pointer_head_path: PathBuf },
}

impl ApplicableDecisions {
    #[must_use]
    pub const fn serving(&self) -> AgentDecisionsServing {
        match self {
            Self::Disabled => AgentDecisionsServing::NotServed,
            Self::Enabled { .. } => AgentDecisionsServing::Served,
        }
    }
}
