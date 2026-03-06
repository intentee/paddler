use std::cmp::Ordering;
use std::hash::Hash;
use std::hash::Hasher;

use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue_severity::AgentIssueSeverity;
use crate::agent_issue_type::AgentIssueType;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentIssue {
    #[serde(rename = "type")]
    pub type_: AgentIssueType,
    pub severity: AgentIssueSeverity,
}

impl PartialEq for AgentIssue {
    fn eq(&self, other: &Self) -> bool {
        self.type_ == other.type_
    }
}

impl Eq for AgentIssue {}

impl Hash for AgentIssue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_.hash(state);
    }
}

impl PartialOrd for AgentIssue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AgentIssue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.type_.cmp(&other.type_)
    }
}

impl From<AgentIssueType> for AgentIssue {
    fn from(type_: AgentIssueType) -> Self {
        let severity = match &type_ {
            AgentIssueType::ChatTemplateDoesNotCompile(_)
            | AgentIssueType::HuggingFaceCannotAcquireLock(_)
            | AgentIssueType::UnableToFindChatTemplate(_)
            | AgentIssueType::HuggingFaceModelDoesNotExist(_)
            | AgentIssueType::ModelCannotBeLoaded(_)
            | AgentIssueType::ModelFileDoesNotExist(_)
            | AgentIssueType::SlotCannotStart(_) => AgentIssueSeverity::Error,
        };

        Self { type_, severity }
    }
}
