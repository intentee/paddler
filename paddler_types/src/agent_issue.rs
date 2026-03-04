use serde::Deserialize;
use serde::Serialize;

use crate::issue_severity::AgentIssueSeverity;
use crate::issue_type::AgentIssueType;

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
            | AgentIssueType::HuggingFaceModelDoesNotExist(_)
            | AgentIssueType::HuggingFacePermissions(_)
            | AgentIssueType::UnableToFindChatTemplate(_)
            | AgentIssueType::ModelCannotBeLoaded(_)
            | AgentIssueType::ModelFileDoesNotExist(_)
            | AgentIssueType::MultimodalProjectionCannotBeLoaded(_)
            | AgentIssueType::SlotCannotStart(_) => AgentIssueSeverity::Error,
        };

        Self { type_, severity }
    }
}
