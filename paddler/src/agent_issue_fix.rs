use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::ModelPath;
use paddler_types::agent_issue_params::SlotCannotStartParams;

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
    #[must_use]
    pub fn can_fix(&self, issue: &AgentIssue) -> bool {
        match issue {
            AgentIssue::ChatTemplateDoesNotCompile(issue_params) => match self {
                Self::ChatTemplateIsCompiled(fix_model_path) => {
                    issue_params.model_path.eq(fix_model_path)
                }
                Self::ModelStateIsReconciled => true,
                _ => false,
            },
            AgentIssue::HuggingFaceCannotAcquireLock(hugging_face_download_lock) => match self {
                Self::HuggingFaceDownloadedModel(fix_model_path)
                | Self::HuggingFaceStartedDownloading(fix_model_path) => {
                    hugging_face_download_lock.model_path.eq(fix_model_path)
                }
                Self::ModelStateIsReconciled => true,
                _ => false,
            },
            AgentIssue::HuggingFaceModelDoesNotExist(issue_model_path)
            | AgentIssue::HuggingFacePermissions(issue_model_path) => match self {
                Self::HuggingFaceDownloadedModel(fix_model_path)
                | Self::HuggingFaceStartedDownloading(fix_model_path)
                | Self::MultimodalProjectionIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                Self::ModelStateIsReconciled => true,
                _ => false,
            },
            AgentIssue::ModelCannotBeLoaded(issue_model_path) => match self {
                Self::ModelIsLoaded(fix_model_path) => issue_model_path.eq(fix_model_path),
                _ => false,
            },
            AgentIssue::ModelFileDoesNotExist(issue_model_path) => match self {
                Self::ModelFileExists(fix_model_path)
                | Self::MultimodalProjectionIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                _ => false,
            },
            AgentIssue::MultimodalProjectionCannotBeLoaded(_) => {
                matches!(self, Self::MultimodalProjectionIsLoaded(_))
            }
            AgentIssue::SlotCannotStart(SlotCannotStartParams {
                error: _,
                slot_index,
            }) => match self {
                Self::SlotStarted(started_slot_index) => slot_index == started_slot_index,
                _ => false,
            },
            AgentIssue::UnableToFindChatTemplate(issue_model_path) => match self {
                Self::ModelChatTemplateIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                Self::ModelStateIsReconciled => true,
                _ => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_types::agent_issue_params::ChatTemplateDoesNotCompileParams;
    use paddler_types::agent_issue_params::SlotCannotStartParams;

    use super::*;

    fn model_path(path: &str) -> ModelPath {
        ModelPath {
            model_path: path.to_owned(),
        }
    }

    #[test]
    fn chat_template_is_compiled_fixes_matching_compile_issue() {
        let fix = AgentIssueFix::ChatTemplateIsCompiled(model_path("model_a"));
        let issue = AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
            error: "syntax error".to_owned(),
            model_path: model_path("model_a"),
            template_content: "template".to_owned(),
        });

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn chat_template_is_compiled_does_not_fix_different_model() {
        let fix = AgentIssueFix::ChatTemplateIsCompiled(model_path("model_a"));
        let issue = AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
            error: "syntax error".to_owned(),
            model_path: model_path("model_b"),
            template_content: "template".to_owned(),
        });

        assert!(!fix.can_fix(&issue));
    }

    #[test]
    fn model_state_is_reconciled_fixes_chat_template_issue() {
        let fix = AgentIssueFix::ModelStateIsReconciled;
        let issue = AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
            error: "error".to_owned(),
            model_path: model_path("any_model"),
            template_content: "template".to_owned(),
        });

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_state_is_reconciled_fixes_unable_to_find_chat_template() {
        let fix = AgentIssueFix::ModelStateIsReconciled;
        let issue = AgentIssue::UnableToFindChatTemplate(model_path("any_model"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn slot_started_matches_by_slot_index() {
        let fix = AgentIssueFix::SlotStarted(3);
        let matching_issue = AgentIssue::SlotCannotStart(SlotCannotStartParams {
            error: "failed".to_owned(),
            slot_index: 3,
        });
        let non_matching_issue = AgentIssue::SlotCannotStart(SlotCannotStartParams {
            error: "failed".to_owned(),
            slot_index: 5,
        });

        assert!(fix.can_fix(&matching_issue));
        assert!(!fix.can_fix(&non_matching_issue));
    }

    #[test]
    fn model_is_loaded_does_not_fix_unrelated_issue() {
        let fix = AgentIssueFix::ModelIsLoaded(model_path("model_a"));
        let issue = AgentIssue::UnableToFindChatTemplate(model_path("model_a"));

        assert!(!fix.can_fix(&issue));
    }

    #[test]
    fn model_is_loaded_fixes_model_cannot_be_loaded_with_same_path() {
        let fix = AgentIssueFix::ModelIsLoaded(model_path("model_a"));
        let issue = AgentIssue::ModelCannotBeLoaded(model_path("model_a"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_file_exists_fixes_model_file_does_not_exist() {
        let fix = AgentIssueFix::ModelFileExists(model_path("model_a"));
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_a"));

        assert!(fix.can_fix(&issue));
    }
}
