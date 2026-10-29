use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::hugging_face_download_failed_params::HuggingFaceDownloadFailedParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::pointer_head_incompatible_with_model_params::PointerHeadIncompatibleWithModelParams;
use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;

#[derive(Debug)]
pub enum AgentIssueFix {
    ChatTemplateIsCompiled(ModelPath),
    HuggingFaceDownloadedModel(ModelPath),
    HuggingFaceStartedDownloading(ModelPath),
    ModelFileExists(ModelPath),
    ModelIsLoaded(ModelPath),
    ModelDownloadCompleted(ModelPath),
    ModelDownloadStarted(ModelPath),
    MultimodalProjectionIsLoaded(ModelPath),
    PointerHeadFileExists(ModelPath),
    PointerHeadIsCompatibleWithModel(ModelPath),
    PointerHeadIsLoaded(ModelPath),
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
                _ => false,
            },
            AgentIssue::HuggingFaceCannotAcquireLock(hugging_face_download_lock) => match self {
                Self::HuggingFaceDownloadedModel(fix_model_path)
                | Self::HuggingFaceStartedDownloading(fix_model_path) => {
                    hugging_face_download_lock.model_path.eq(fix_model_path)
                }
                _ => false,
            },
            AgentIssue::HuggingFaceDownloadFailed(HuggingFaceDownloadFailedParams {
                failure: _,
                model_path: issue_model_path,
            }) => match self {
                Self::HuggingFaceDownloadedModel(fix_model_path)
                | Self::HuggingFaceStartedDownloading(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                _ => false,
            },
            AgentIssue::HuggingFaceModelDoesNotExist(issue_model_path)
            | AgentIssue::HuggingFacePermissions(issue_model_path) => match self {
                Self::HuggingFaceDownloadedModel(fix_model_path)
                | Self::HuggingFaceStartedDownloading(fix_model_path)
                | Self::MultimodalProjectionIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                _ => false,
            },
            AgentIssue::ModelCannotBeLoaded(issue_model_path) => match self {
                Self::ModelIsLoaded(fix_model_path) => issue_model_path.eq(fix_model_path),
                _ => false,
            },
            AgentIssue::LocalModelFileCannotBeChecked(issue_model_path) => match self {
                Self::ModelFileExists(fix_model_path)
                | Self::MultimodalProjectionIsLoaded(fix_model_path)
                | Self::PointerHeadFileExists(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                _ => false,
            },
            AgentIssue::ModelFileDoesNotExist(issue_model_path) => match self {
                Self::ModelFileExists(fix_model_path)
                | Self::MultimodalProjectionIsLoaded(fix_model_path) => {
                    issue_model_path.eq(fix_model_path)
                }
                _ => false,
            },
            AgentIssue::AgentModelUriIsNotAnAbsoluteLocalPath(_)
            | AgentIssue::AgentModelUriIsNotCanonical(_)
            | AgentIssue::HuggingFaceModelUriIsMalformed(_)
            | AgentIssue::ModelUriIsUnparseable(_)
            | AgentIssue::SlotsInsufficientForDecisions(_)
            | AgentIssue::UnableToFindChatTemplate(_) => false,
            AgentIssue::MultimodalProjectionCannotBeLoaded(_) => {
                matches!(self, Self::MultimodalProjectionIsLoaded(_))
            }
            AgentIssue::PointerHeadCannotBeLoaded(issue_pointer_head_path) => match self {
                Self::PointerHeadFileExists(fix_pointer_head_path)
                | Self::PointerHeadIsLoaded(fix_pointer_head_path) => {
                    issue_pointer_head_path.eq(fix_pointer_head_path)
                }
                _ => false,
            },
            AgentIssue::PointerHeadIncompatibleWithModel(
                PointerHeadIncompatibleWithModelParams {
                    incompatibility: _,
                    pointer_head_path: issue_pointer_head_path,
                },
            ) => match self {
                Self::PointerHeadIsCompatibleWithModel(fix_pointer_head_path) => {
                    issue_pointer_head_path.eq(fix_pointer_head_path)
                }
                _ => false,
            },
            AgentIssue::SlotCannotStart(SlotCannotStartParams {
                error: _,
                inference_capability: _,
                slot_index,
            }) => match self {
                Self::SlotStarted(started_slot_index) => slot_index == started_slot_index,
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
                _ => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::agent_model_uri_not_canonical_params::AgentModelUriNotCanonicalParams;
    use paddler_messaging::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
    use paddler_messaging::agent_issue_params::hugging_face_download_failed_params::HuggingFaceDownloadFailedParams;
    use paddler_messaging::agent_issue_params::hugging_face_download_failure::HuggingFaceDownloadFailure;
    use paddler_messaging::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;
    use paddler_messaging::agent_issue_params::pointer_head_incompatible_with_model_params::PointerHeadIncompatibleWithModelParams;
    use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
    use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
    use paddler_messaging::inference_capability::InferenceCapability;

    use super::AgentIssueFix;

    fn model_path(path: &str) -> ModelPath {
        ModelPath {
            model_path: path.to_owned(),
        }
    }

    #[test]
    fn configuration_issues_are_cleared_only_with_their_scope() {
        let issue_path = model_path("model_a");
        let configuration_issues = [
            AgentIssue::AgentModelUriIsNotAnAbsoluteLocalPath(issue_path.clone()),
            AgentIssue::AgentModelUriIsNotCanonical(AgentModelUriNotCanonicalParams {
                canonical_uri: "agent:///model_a".to_owned(),
                model_path: issue_path.clone(),
            }),
            AgentIssue::HuggingFaceModelUriIsMalformed(issue_path.clone()),
            AgentIssue::ModelUriIsUnparseable(issue_path.clone()),
            AgentIssue::SlotsInsufficientForDecisions(SlotsInsufficientForDecisionsParams {
                desired_slots: 1,
                required_slots: 2,
            }),
            AgentIssue::UnableToFindChatTemplate(issue_path.clone()),
        ];
        let targeted_fixes = [
            AgentIssueFix::ChatTemplateIsCompiled(issue_path.clone()),
            AgentIssueFix::HuggingFaceDownloadedModel(issue_path.clone()),
            AgentIssueFix::HuggingFaceStartedDownloading(issue_path.clone()),
            AgentIssueFix::ModelDownloadCompleted(issue_path.clone()),
            AgentIssueFix::ModelDownloadStarted(issue_path.clone()),
            AgentIssueFix::ModelFileExists(issue_path.clone()),
            AgentIssueFix::ModelIsLoaded(issue_path.clone()),
            AgentIssueFix::MultimodalProjectionIsLoaded(issue_path.clone()),
            AgentIssueFix::PointerHeadFileExists(issue_path.clone()),
            AgentIssueFix::PointerHeadIsCompatibleWithModel(issue_path.clone()),
            AgentIssueFix::PointerHeadIsLoaded(issue_path),
            AgentIssueFix::SlotStarted(0),
        ];

        for issue in &configuration_issues {
            for fix in &targeted_fixes {
                assert!(!fix.can_fix(issue), "{fix:?} must not fix {issue:?}");
            }
        }
    }

    #[test]
    fn pointer_head_file_exists_fixes_only_the_pointer_head_it_names() {
        let fix = AgentIssueFix::PointerHeadFileExists(model_path("pointer_head_a"));

        assert!(
            fix.can_fix(&AgentIssue::PointerHeadCannotBeLoaded(model_path(
                "pointer_head_a"
            )))
        );
        assert!(
            !fix.can_fix(&AgentIssue::PointerHeadCannotBeLoaded(model_path(
                "pointer_head_b"
            )))
        );
    }

    #[test]
    fn pointer_head_is_loaded_fixes_only_the_pointer_head_it_names() {
        let fix = AgentIssueFix::PointerHeadIsLoaded(model_path("pointer_head_a"));

        assert!(
            fix.can_fix(&AgentIssue::PointerHeadCannotBeLoaded(model_path(
                "pointer_head_a"
            )))
        );
        assert!(
            !fix.can_fix(&AgentIssue::PointerHeadCannotBeLoaded(model_path(
                "pointer_head_b"
            )))
        );
    }

    #[test]
    fn pointer_head_is_compatible_with_model_fixes_only_the_incompatibility_of_the_pointer_head_it_names()
     {
        let fix = AgentIssueFix::PointerHeadIsCompatibleWithModel(model_path("pointer_head_a"));
        let incompatibility_of = |pointer_head_path: &str| {
            AgentIssue::PointerHeadIncompatibleWithModel(PointerHeadIncompatibleWithModelParams {
                incompatibility: PointerHeadIncompatibility::HiddenSizeMismatch {
                    model_hidden_size: 1024,
                    pointer_head_hidden_size: 512,
                },
                pointer_head_path: model_path(pointer_head_path),
            })
        };

        assert!(fix.can_fix(&incompatibility_of("pointer_head_a")));
        assert!(!fix.can_fix(&incompatibility_of("pointer_head_b")));
        assert!(
            !fix.can_fix(&AgentIssue::PointerHeadCannotBeLoaded(model_path(
                "pointer_head_a"
            )))
        );
    }

    #[test]
    fn a_local_model_file_that_cannot_be_checked_is_cleared_once_that_file_is_found() {
        let found_file_fixes = [
            AgentIssueFix::ModelFileExists(model_path("model_a")),
            AgentIssueFix::MultimodalProjectionIsLoaded(model_path("model_a")),
            AgentIssueFix::PointerHeadFileExists(model_path("model_a")),
        ];

        for fix in &found_file_fixes {
            assert!(
                fix.can_fix(&AgentIssue::LocalModelFileCannotBeChecked(model_path(
                    "model_a"
                )))
            );
            assert!(
                !fix.can_fix(&AgentIssue::LocalModelFileCannotBeChecked(model_path(
                    "model_b"
                )))
            );
        }
    }

    #[test]
    fn a_local_model_file_that_cannot_be_checked_is_not_cleared_by_fixes_that_do_not_find_it() {
        let issue = AgentIssue::LocalModelFileCannotBeChecked(model_path("model_a"));
        let unrelated_fixes = [
            AgentIssueFix::ChatTemplateIsCompiled(model_path("model_a")),
            AgentIssueFix::HuggingFaceDownloadedModel(model_path("model_a")),
            AgentIssueFix::HuggingFaceStartedDownloading(model_path("model_a")),
            AgentIssueFix::ModelDownloadCompleted(model_path("model_a")),
            AgentIssueFix::ModelDownloadStarted(model_path("model_a")),
            AgentIssueFix::ModelIsLoaded(model_path("model_a")),
            AgentIssueFix::PointerHeadIsCompatibleWithModel(model_path("model_a")),
            AgentIssueFix::PointerHeadIsLoaded(model_path("model_a")),
            AgentIssueFix::SlotStarted(0),
        ];

        for fix in &unrelated_fixes {
            assert!(!fix.can_fix(&issue), "{fix:?} must not fix {issue:?}");
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
    fn slot_started_matches_by_slot_index() {
        let fix = AgentIssueFix::SlotStarted(3);
        let matching_issue = AgentIssue::SlotCannotStart(SlotCannotStartParams {
            error: "failed".to_owned(),
            inference_capability: InferenceCapability::TextGeneration,
            slot_index: 3,
        });
        let non_matching_issue = AgentIssue::SlotCannotStart(SlotCannotStartParams {
            error: "failed".to_owned(),
            inference_capability: InferenceCapability::Decisions,
            slot_index: 5,
        });

        assert!(fix.can_fix(&matching_issue));
        assert!(!fix.can_fix(&non_matching_issue));
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
        assert!(
            !AgentIssueFix::HuggingFaceStartedDownloading(model_path("model_b")).can_fix(&issue)
        );
        assert!(!AgentIssueFix::ModelIsLoaded(model_path("model_a")).can_fix(&issue));
    }

    #[test]
    fn a_failed_hugging_face_download_is_fixed_only_by_downloading_the_same_model() {
        let issue = AgentIssue::HuggingFaceDownloadFailed(HuggingFaceDownloadFailedParams {
            failure: HuggingFaceDownloadFailure::HubConnectionFailed,
            model_path: model_path("model_a"),
        });

        assert!(
            AgentIssueFix::HuggingFaceStartedDownloading(model_path("model_a")).can_fix(&issue)
        );
        assert!(AgentIssueFix::HuggingFaceDownloadedModel(model_path("model_a")).can_fix(&issue));
        assert!(
            !AgentIssueFix::HuggingFaceStartedDownloading(model_path("model_b")).can_fix(&issue)
        );
        assert!(!AgentIssueFix::ModelDownloadStarted(model_path("model_a")).can_fix(&issue));
    }

    #[test]
    fn hugging_face_model_does_not_exist_fixes() {
        let issue = AgentIssue::HuggingFaceModelDoesNotExist(model_path("model_a"));

        assert!(AgentIssueFix::HuggingFaceDownloadedModel(model_path("model_a")).can_fix(&issue));
        assert!(AgentIssueFix::MultimodalProjectionIsLoaded(model_path("model_a")).can_fix(&issue));
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
            inference_capability: InferenceCapability::Embeddings,
            slot_index: 1,
        });

        assert!(!fix.can_fix(&issue));
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
