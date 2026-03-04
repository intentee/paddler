use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::SlotCannotStartParams;
use paddler_types::issue_type::IssueType;

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
            IssueType::ChatTemplateDoesNotCompile(_) => matches!(
                self,
                AgentIssueFix::ChatTemplateIsCompiled | AgentIssueFix::ModelStateIsReconciled
            ),
            IssueType::HuggingFaceCannotAcquireLock(_) => matches!(
                self,
                AgentIssueFix::HuggingFaceDownloadedModel
                    | AgentIssueFix::HuggingFaceStartedDownloading
                    | AgentIssueFix::ModelStateIsReconciled
            ),
            IssueType::HuggingFaceModelDoesNotExist(_) => matches!(
                self,
                AgentIssueFix::HuggingFaceDownloadedModel
                    | AgentIssueFix::HuggingFaceStartedDownloading
                    | AgentIssueFix::ModelStateIsReconciled
            ),
            IssueType::ModelCannotBeLoaded(_) => matches!(self, AgentIssueFix::ModelIsLoaded),
            IssueType::ModelFileDoesNotExist(_) => matches!(self, AgentIssueFix::ModelFileExists),
            IssueType::SlotCannotStart(SlotCannotStartParams {
                error: _,
                slot_index,
            }) => match self {
                AgentIssueFix::SlotStarted(started_slot_index) => started_slot_index == slot_index,
                _ => false,
            },
            IssueType::UnableToFindChatTemplate(_) => matches!(
                self,
                AgentIssueFix::ModelChatTemplateIsLoaded | AgentIssueFix::ModelStateIsReconciled
            ),
        }
    }
}
