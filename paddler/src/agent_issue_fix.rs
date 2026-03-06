use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::ModelPath;
use paddler_types::agent_issue_params::SlotCannotStartParams;
use paddler_types::agent_issue_type::AgentIssueType;

#[derive(Debug)]
pub enum AgentIssueFix {
    ChatTemplateIsCompiled(ModelPath),
    HuggingFaceDownloadedModel(ModelPath),
    HuggingFaceStartedDownloading(ModelPath),
    ModelChatTemplateIsLoaded(ModelPath),
    ModelFileExists(ModelPath),
    ModelIsLoaded(ModelPath),
    ModelStateIsReconciled,
    MultimodalProjectionIsLoaded(ModelPath),
    SlotStarted(u32),
}

impl AgentIssueFix {
    pub fn can_fix(&self, issue: &AgentIssue) -> bool {
        match &issue.type_ {
            AgentIssueType::ChatTemplateDoesNotCompile(issue_params) => match self {
                AgentIssueFix::ChatTemplateIsCompiled(fix_model_path) => {
                    issue_params.model_path.eq(fix_model_path)
                }
                AgentIssueFix::ModelStateIsReconciled => true,
                _ => false,
            },
            AgentIssueType::HuggingFaceCannotAcquireLock(hugging_face_download_lock) => {
                match self {
                    AgentIssueFix::HuggingFaceDownloadedModel(fix_model_path)
                    | AgentIssueFix::HuggingFaceStartedDownloading(fix_model_path) => {
                        hugging_face_download_lock.model_path.eq(fix_model_path)
                    }
                    AgentIssueFix::ModelStateIsReconciled => true,
                    _ => false,
                }
            }
            AgentIssueType::HuggingFaceModelDoesNotExist(issue_model_path)
            | AgentIssueType::HuggingFacePermissions(issue_model_path) => match self {
                AgentIssueFix::HuggingFaceDownloadedModel(fix_model_path)
                | AgentIssueFix::HuggingFaceStartedDownloading(fix_model_path)
                | AgentIssueFix::MultimodalProjectionIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                AgentIssueFix::ModelStateIsReconciled => true,
                _ => false,
            },
            AgentIssueType::ModelCannotBeLoaded(issue_model_path) => match self {
                AgentIssueFix::ModelIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                _ => false,
            },
            AgentIssueType::ModelFileDoesNotExist(issue_model_path) => match self {
                AgentIssueFix::ModelFileExists(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                AgentIssueFix::MultimodalProjectionIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                _ => false,
            },
            AgentIssueType::MultimodalProjectionCannotBeLoaded(_) => {
                matches!(self, AgentIssueFix::MultimodalProjectionIsLoaded(_))
            }
            AgentIssueType::SlotCannotStart(SlotCannotStartParams {
                error: _,
                slot_index,
            }) => match self {
                AgentIssueFix::SlotStarted(started_slot_index) => {
                    *started_slot_index == *slot_index
                }
                _ => false,
            },
            AgentIssueType::UnableToFindChatTemplate(issue_model_path) => match self {
                AgentIssueFix::ModelChatTemplateIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                AgentIssueFix::ModelStateIsReconciled => true,
                _ => false,
            },
        }
    }
}
