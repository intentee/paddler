use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;

#[derive(Debug)]
pub enum AgentIssueFix {
    ChatTemplateIsCompiled(ModelPath),
    DesiredStateIsReplaced,
    HuggingFaceDownloadedModel(ModelPath),
    HuggingFaceStartedDownloading(ModelPath),
    ModelChatTemplateIsLoaded(ModelPath),
    ModelFileExists(ModelPath),
    ModelIsLoaded(ModelPath),
    ModelDownloadCompleted(ModelPath),
    ModelDownloadStarted(ModelPath),
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
                Self::DesiredStateIsReplaced | Self::ModelStateIsReconciled => true,
                _ => false,
            },
            AgentIssue::HuggingFaceModelDoesNotExist(issue_model_path)
            | AgentIssue::HuggingFacePermissions(issue_model_path) => match self {
                Self::HuggingFaceDownloadedModel(fix_model_path)
                | Self::HuggingFaceStartedDownloading(fix_model_path)
                | Self::MultimodalProjectionIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                Self::DesiredStateIsReplaced | Self::ModelStateIsReconciled => true,
                _ => false,
            },
            AgentIssue::ModelCannotBeLoaded(issue_model_path) => match self {
                Self::ModelIsLoaded(fix_model_path) => issue_model_path.eq(fix_model_path),
                _ => false,
            },
            AgentIssue::ModelFileDoesNotExist(issue_model_path) => match self {
                Self::DesiredStateIsReplaced => true,
                Self::ModelFileExists(fix_model_path)
                | Self::MultimodalProjectionIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                _ => false,
            },
            AgentIssue::HuggingFaceModelUriIsMalformed(_)
            | AgentIssue::ModelUriIsUnparseable(_)
            | AgentIssue::PointerHeadCannotBeLoaded(_) => {
                matches!(
                    self,
                    Self::DesiredStateIsReplaced | Self::ModelStateIsReconciled
                )
            }
            AgentIssue::ModelArchitectureUnsupportedForDecisions(_)
            | AgentIssue::PointerHeadIncompatibleWithModel(_)
            | AgentIssue::SlotsInsufficientForDecisions(_) => {
                matches!(self, Self::ModelStateIsReconciled)
            }
            AgentIssue::MultimodalProjectionCannotBeLoaded(_) => {
                matches!(
                    self,
                    Self::DesiredStateIsReplaced | Self::MultimodalProjectionIsLoaded(_)
                )
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
            AgentIssue::CacheCannotAcquireLock(issue_model_path)
            | AgentIssue::CacheDirectoryIsNotWritable(issue_model_path)
            | AgentIssue::CacheStorageIsFull(issue_model_path)
            | AgentIssue::DownloadInterrupted(issue_model_path)
            | AgentIssue::DownloadServerDeniedAccess(issue_model_path)
            | AgentIssue::DownloadServerErrored(issue_model_path)
            | AgentIssue::DownloadServerIsUnreachable(issue_model_path)
            | AgentIssue::DownloadServerRejectedRequest(issue_model_path)
            | AgentIssue::DownloadUrlIsMalformed(issue_model_path)
            | AgentIssue::ModelCacheIsCorrupted(issue_model_path)
            | AgentIssue::ModelDoesNotExistAtUrl(issue_model_path) => match self {
                Self::ModelDownloadCompleted(fix_model_path)
                | Self::ModelDownloadStarted(fix_model_path) => issue_model_path.eq(fix_model_path),
                Self::DesiredStateIsReplaced | Self::ModelStateIsReconciled => true,
                _ => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
    use paddler_messaging::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
    use paddler_messaging::agent_issue_params::model_architecture_unsupported_for_decisions_params::ModelArchitectureUnsupportedForDecisionsParams;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;
    use paddler_messaging::agent_issue_params::pointer_head_incompatible_with_model_params::PointerHeadIncompatibleWithModelParams;
    use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
    use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;

    use super::AgentIssueFix;

    fn model_path(path: &str) -> ModelPath {
        ModelPath {
            model_path: path.to_owned(),
        }
    }

    #[test]
    fn only_reconciling_the_state_fixes_decision_pipeline_issues() {
        let decision_pipeline_issues = [
            AgentIssue::ModelArchitectureUnsupportedForDecisions(
                ModelArchitectureUnsupportedForDecisionsParams {
                    architecture: "qwen3".to_owned(),
                    model_path: model_path("model_a"),
                },
            ),
            AgentIssue::PointerHeadIncompatibleWithModel(PointerHeadIncompatibleWithModelParams {
                incompatibility: PointerHeadIncompatibility::HiddenSizeMismatch {
                    model_hidden_size: 1024,
                    pointer_head_hidden_size: 512,
                },
                pointer_head_path: model_path("pointer_head"),
            }),
            AgentIssue::SlotsInsufficientForDecisions(SlotsInsufficientForDecisionsParams {
                desired_slots: 1,
                required_slots: 2,
            }),
        ];

        for issue in &decision_pipeline_issues {
            assert!(AgentIssueFix::ModelStateIsReconciled.can_fix(issue));
            assert!(!AgentIssueFix::DesiredStateIsReplaced.can_fix(issue));
            assert!(!AgentIssueFix::ModelIsLoaded(model_path("model_a")).can_fix(issue));
        }
    }

    #[test]
    fn reconciling_the_state_fixes_model_reference_issues() {
        let model_reference_issues = [
            AgentIssue::HuggingFaceModelUriIsMalformed(model_path("https://huggingface.co/owner")),
            AgentIssue::ModelUriIsUnparseable(model_path("not a uri")),
            AgentIssue::PointerHeadCannotBeLoaded(model_path("pointer_head")),
        ];

        for issue in &model_reference_issues {
            assert!(AgentIssueFix::ModelStateIsReconciled.can_fix(issue));
            assert!(!AgentIssueFix::ModelIsLoaded(model_path("model_a")).can_fix(issue));
        }
    }

    #[test]
    fn replacing_the_desired_state_fixes_the_issues_raised_while_resolving_it() {
        let download_url = model_path("https://example.com/m.gguf");
        let resolution_issues = [
            AgentIssue::CacheCannotAcquireLock(download_url.clone()),
            AgentIssue::CacheDirectoryIsNotWritable(download_url.clone()),
            AgentIssue::CacheStorageIsFull(download_url.clone()),
            AgentIssue::DownloadInterrupted(download_url.clone()),
            AgentIssue::DownloadServerDeniedAccess(download_url.clone()),
            AgentIssue::DownloadServerErrored(download_url.clone()),
            AgentIssue::DownloadServerIsUnreachable(download_url.clone()),
            AgentIssue::DownloadServerRejectedRequest(download_url.clone()),
            AgentIssue::DownloadUrlIsMalformed(download_url.clone()),
            AgentIssue::HuggingFaceCannotAcquireLock(HuggingFaceDownloadLock {
                lock_path: "/cache/model.lock".to_owned(),
                model_path: model_path("owner/repo/model.gguf"),
            }),
            AgentIssue::HuggingFaceModelDoesNotExist(model_path("owner/repo/model.gguf")),
            AgentIssue::HuggingFaceModelUriIsMalformed(model_path("https://huggingface.co/owner")),
            AgentIssue::HuggingFacePermissions(model_path("owner/repo/model.gguf")),
            AgentIssue::ModelCacheIsCorrupted(download_url.clone()),
            AgentIssue::ModelDoesNotExistAtUrl(download_url),
            AgentIssue::ModelFileDoesNotExist(model_path("/models/model.gguf")),
            AgentIssue::ModelUriIsUnparseable(model_path("not a uri")),
            AgentIssue::MultimodalProjectionCannotBeLoaded(model_path("/models/mmproj.gguf")),
            AgentIssue::PointerHeadCannotBeLoaded(model_path("/models/pointer_head.gguf")),
        ];

        for issue in &resolution_issues {
            assert!(AgentIssueFix::DesiredStateIsReplaced.can_fix(issue));
        }
    }

    #[test]
    fn replacing_the_desired_state_keeps_the_issues_of_the_running_pipeline() {
        let running_pipeline_issues = [
            AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
                error: "syntax error".to_owned(),
                model_path: model_path("model_a"),
                template_content: "template".to_owned(),
            }),
            AgentIssue::ModelCannotBeLoaded(model_path("model_a")),
            AgentIssue::SlotCannotStart(SlotCannotStartParams {
                error: "context exhausted".to_owned(),
                slot_index: 0,
            }),
            AgentIssue::UnableToFindChatTemplate(model_path("model_a")),
        ];

        for issue in &running_pipeline_issues {
            assert!(!AgentIssueFix::DesiredStateIsReplaced.can_fix(issue));
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

    #[test]
    fn model_download_completed_fixes_model_does_not_exist_at_url_with_same_path() {
        let fix = AgentIssueFix::ModelDownloadCompleted(model_path("https://example.com/m.gguf"));
        let issue = AgentIssue::ModelDoesNotExistAtUrl(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_started_fixes_download_server_denied_access() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue =
            AgentIssue::DownloadServerDeniedAccess(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_started_fixes_cache_directory_is_not_writable() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue =
            AgentIssue::CacheDirectoryIsNotWritable(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_started_fixes_cache_storage_is_full() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue = AgentIssue::CacheStorageIsFull(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_started_fixes_download_server_is_unreachable() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue =
            AgentIssue::DownloadServerIsUnreachable(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_started_fixes_download_url_is_malformed() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue = AgentIssue::DownloadUrlIsMalformed(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_started_fixes_model_cache_is_corrupted() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue = AgentIssue::ModelCacheIsCorrupted(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_started_fixes_download_server_errored() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue = AgentIssue::DownloadServerErrored(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_started_fixes_download_interrupted() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue = AgentIssue::DownloadInterrupted(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_completed_does_not_fix_different_url() {
        let fix = AgentIssueFix::ModelDownloadCompleted(model_path("https://example.com/a.gguf"));
        let issue = AgentIssue::ModelDoesNotExistAtUrl(model_path("https://example.com/b.gguf"));

        assert!(!fix.can_fix(&issue));
    }

    #[test]
    fn model_state_is_reconciled_fixes_model_cache_is_corrupted() {
        let fix = AgentIssueFix::ModelStateIsReconciled;
        let issue = AgentIssue::ModelCacheIsCorrupted(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_started_fixes_cache_cannot_acquire_lock() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue = AgentIssue::CacheCannotAcquireLock(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_completed_fixes_cache_cannot_acquire_lock() {
        let fix = AgentIssueFix::ModelDownloadCompleted(model_path("https://example.com/m.gguf"));
        let issue = AgentIssue::CacheCannotAcquireLock(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_download_started_does_not_fix_huggingface_issues() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue =
            AgentIssue::HuggingFaceModelDoesNotExist(model_path("https://example.com/m.gguf"));

        assert!(!fix.can_fix(&issue));
    }

    #[test]
    fn chat_template_does_not_compile_not_fixed_by_unrelated_fix() {
        let fix = AgentIssueFix::ModelIsLoaded(model_path("model_a"));
        let issue = AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
            error: "error".to_owned(),
            model_path: model_path("model_a"),
            template_content: "template".to_owned(),
        });

        assert!(!fix.can_fix(&issue));
    }

    #[test]
    fn hugging_face_cannot_acquire_lock_fixes() {
        let issue = AgentIssue::HuggingFaceCannotAcquireLock(HuggingFaceDownloadLock {
            lock_path: "/tmp/lock".to_owned(),
            model_path: model_path("model_a"),
        });

        assert!(AgentIssueFix::HuggingFaceDownloadedModel(model_path("model_a")).can_fix(&issue));
        assert!(
            AgentIssueFix::HuggingFaceStartedDownloading(model_path("model_a")).can_fix(&issue)
        );
        assert!(AgentIssueFix::ModelStateIsReconciled.can_fix(&issue));
        assert!(
            !AgentIssueFix::HuggingFaceStartedDownloading(model_path("model_b")).can_fix(&issue)
        );
        assert!(!AgentIssueFix::ModelIsLoaded(model_path("model_a")).can_fix(&issue));
    }

    #[test]
    fn hugging_face_model_does_not_exist_fixes() {
        let issue = AgentIssue::HuggingFaceModelDoesNotExist(model_path("model_a"));

        assert!(AgentIssueFix::HuggingFaceDownloadedModel(model_path("model_a")).can_fix(&issue));
        assert!(AgentIssueFix::MultimodalProjectionIsLoaded(model_path("model_a")).can_fix(&issue));
        assert!(AgentIssueFix::ModelStateIsReconciled.can_fix(&issue));
        assert!(!AgentIssueFix::ModelIsLoaded(model_path("model_a")).can_fix(&issue));
    }

    #[test]
    fn hugging_face_permissions_fixed_by_started_downloading() {
        let fix = AgentIssueFix::HuggingFaceStartedDownloading(model_path("model_a"));
        let issue = AgentIssue::HuggingFacePermissions(model_path("model_a"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn model_cannot_be_loaded_not_fixed_by_unrelated_fix() {
        let fix = AgentIssueFix::ModelFileExists(model_path("model_a"));
        let issue = AgentIssue::ModelCannotBeLoaded(model_path("model_a"));

        assert!(!fix.can_fix(&issue));
    }

    #[test]
    fn model_file_does_not_exist_fixed_by_multimodal_projection_and_not_others() {
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_a"));

        assert!(AgentIssueFix::MultimodalProjectionIsLoaded(model_path("model_a")).can_fix(&issue));
        assert!(!AgentIssueFix::ModelIsLoaded(model_path("model_a")).can_fix(&issue));
    }

    #[test]
    fn multimodal_projection_cannot_be_loaded_fixed_only_by_multimodal_projection_loaded() {
        let issue = AgentIssue::MultimodalProjectionCannotBeLoaded(model_path("model_a"));

        assert!(AgentIssueFix::MultimodalProjectionIsLoaded(model_path("model_a")).can_fix(&issue));
        assert!(!AgentIssueFix::ModelIsLoaded(model_path("model_a")).can_fix(&issue));
    }

    #[test]
    fn slot_cannot_start_not_fixed_by_unrelated_fix() {
        let fix = AgentIssueFix::ModelIsLoaded(model_path("model_a"));
        let issue = AgentIssue::SlotCannotStart(SlotCannotStartParams {
            error: "failed".to_owned(),
            slot_index: 1,
        });

        assert!(!fix.can_fix(&issue));
    }

    #[test]
    fn unable_to_find_chat_template_fixed_by_model_chat_template_loaded() {
        let issue = AgentIssue::UnableToFindChatTemplate(model_path("model_a"));

        assert!(AgentIssueFix::ModelChatTemplateIsLoaded(model_path("model_a")).can_fix(&issue));
        assert!(!AgentIssueFix::ModelChatTemplateIsLoaded(model_path("model_b")).can_fix(&issue));
    }

    #[test]
    fn download_server_rejected_request_fixed_by_model_download_started() {
        let fix = AgentIssueFix::ModelDownloadStarted(model_path("https://example.com/m.gguf"));
        let issue =
            AgentIssue::DownloadServerRejectedRequest(model_path("https://example.com/m.gguf"));

        assert!(fix.can_fix(&issue));
    }

    #[test]
    fn download_issue_not_fixed_by_unrelated_fix() {
        let fix = AgentIssueFix::ModelIsLoaded(model_path("https://example.com/m.gguf"));
        let issue = AgentIssue::DownloadInterrupted(model_path("https://example.com/m.gguf"));

        assert!(!fix.can_fix(&issue));
    }
}
