use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::SlotCannotStartParams;
use paddler_types::agent_issue_type::AgentIssueType;

pub enum AgentIssueFix {
    ChatTemplateIsCompiled,
    HuggingFaceDownloadedModel,
    HuggingFaceStartedDownloading,
    ModelChatTemplateIsLoaded,
    ModelFileExists,
    ModelIsLoaded,
    ModelStateIsReconciled,
    SlotStarted(u32),
}

impl AgentIssueFix {
    pub fn can_fix(&self, issue: &AgentIssue) -> bool {
        match &issue.type_ {
            AgentIssueType::ChatTemplateDoesNotCompile(_) => matches!(
                self,
                AgentIssueFix::ChatTemplateIsCompiled | AgentIssueFix::ModelStateIsReconciled
            ),
            AgentIssueType::HuggingFaceCannotAcquireLock(_) => matches!(
                self,
                AgentIssueFix::HuggingFaceDownloadedModel
                    | AgentIssueFix::HuggingFaceStartedDownloading
                    | AgentIssueFix::ModelStateIsReconciled
            ),
            AgentIssueType::HuggingFaceModelDoesNotExist(_) => matches!(
                self,
                AgentIssueFix::HuggingFaceDownloadedModel
                    | AgentIssueFix::HuggingFaceStartedDownloading
                    | AgentIssueFix::ModelStateIsReconciled
            ),
            AgentIssueType::ModelCannotBeLoaded(_) => matches!(self, AgentIssueFix::ModelIsLoaded),
            AgentIssueType::ModelFileDoesNotExist(_) => {
                matches!(self, AgentIssueFix::ModelFileExists)
            }
            AgentIssueType::SlotCannotStart(SlotCannotStartParams {
                error: _,
                slot_index,
            }) => match self {
                AgentIssueFix::SlotStarted(started_slot_index) => started_slot_index == slot_index,
                _ => false,
            },
            AgentIssueType::UnableToFindChatTemplate(_) => matches!(
                self,
                AgentIssueFix::ModelChatTemplateIsLoaded | AgentIssueFix::ModelStateIsReconciled
            ),
        }
    }
}
