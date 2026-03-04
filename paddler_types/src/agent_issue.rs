use serde::Deserialize;
use serde::Serialize;

use crate::issue_severity::IssueSeverity;
use crate::issue_type::IssueType;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentIssue {
    #[serde(rename = "type")]
    pub type_: IssueType,
    pub severity: IssueSeverity,
}

impl From<IssueType> for AgentIssue {
    fn from(type_: IssueType) -> Self {
        let severity = match &type_ {
            IssueType::ChatTemplateDoesNotCompile(_)
            | IssueType::HuggingFaceCannotAcquireLock(_)
            | IssueType::UnableToFindChatTemplate(_)
            | IssueType::HuggingFaceModelDoesNotExist(_)
            | IssueType::ModelCannotBeLoaded(_)
            | IssueType::ModelFileDoesNotExist(_)
            | IssueType::SlotCannotStart(_) => IssueSeverity::Error,
        };

        Self { type_, severity }
    }
}
