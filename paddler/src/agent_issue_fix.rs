use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::SlotCannotStartParams;

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
        match issue {
            AgentIssue::ChatTemplateDoesNotCompile(_) => matches!(
                self,
                AgentIssueFix::ChatTemplateIsCompiled | AgentIssueFix::ModelStateIsReconciled
            ),
            AgentIssue::HuggingFaceCannotAcquireLock(_) => matches!(
                self,
                AgentIssueFix::HuggingFaceDownloadedModel
                    | AgentIssueFix::HuggingFaceStartedDownloading
                    | AgentIssueFix::ModelStateIsReconciled
            ),
            AgentIssue::HuggingFaceModelDoesNotExist(_) => matches!(
                self,
                AgentIssueFix::HuggingFaceDownloadedModel
                    | AgentIssueFix::HuggingFaceStartedDownloading
                    | AgentIssueFix::ModelStateIsReconciled
            ),
            AgentIssue::ModelCannotBeLoaded(_) => matches!(self, AgentIssueFix::ModelIsLoaded),
            AgentIssue::ModelFileDoesNotExist(_) => matches!(self, AgentIssueFix::ModelFileExists),
            AgentIssue::SlotCannotStart(SlotCannotStartParams {
                error: _,
                slot_index,
            }) => match self {
                AgentIssueFix::SlotStarted(started_slot_index) => started_slot_index == slot_index,
                _ => false,
            },
            AgentIssue::UnableToFindChatTemplate(_) => matches!(
                self,
                AgentIssueFix::ModelChatTemplateIsLoaded | AgentIssueFix::ModelStateIsReconciled
            ),
        }
    }
}
