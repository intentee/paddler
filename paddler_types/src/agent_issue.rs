use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue_severity::AgentIssueSeverity;
use crate::agent_issue_type::AgentIssueType;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentIssue {
    #[serde(rename = "type")]
    pub type_: AgentIssueType,
    pub severity: AgentIssueSeverity,
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
