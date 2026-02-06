use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue_params::ChatTemplateDoesNotCompileParams;
use crate::agent_issue_params::SlotCannotStartParams;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentIssue {
    ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams),
    HuggingFaceCannotAcquireLock(String),
    HuggingFaceModelDoesNotExist(String),
    ModelCannotBeLoaded(String),
    ModelFileDoesNotExist(String),
    SlotCannotStart(SlotCannotStartParams),
    UnableToFindChatTemplate(String),
}
