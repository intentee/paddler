use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue_params::agent_model_uri_not_canonical_params::AgentModelUriNotCanonicalParams;
use crate::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
use crate::agent_issue_params::hugging_face_download_failed_params::HuggingFaceDownloadFailedParams;
use crate::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
use crate::agent_issue_params::model_path::ModelPath;
use crate::agent_issue_params::pointer_head_incompatible_with_model_params::PointerHeadIncompatibleWithModelParams;
use crate::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
use crate::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentIssue {
    AgentModelUriIsNotAnAbsoluteLocalPath(ModelPath),
    AgentModelUriIsNotCanonical(AgentModelUriNotCanonicalParams),
    CacheCannotAcquireLock(ModelPath),
    CacheDirectoryIsNotWritable(ModelPath),
    CacheStorageIsFull(ModelPath),
    ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams),
    DownloadInterrupted(ModelPath),
    DownloadServerDeniedAccess(ModelPath),
    DownloadServerErrored(ModelPath),
    DownloadServerIsUnreachable(ModelPath),
    DownloadServerRejectedRequest(ModelPath),
    DownloadUrlIsMalformed(ModelPath),
    HuggingFaceCannotAcquireLock(HuggingFaceDownloadLock),
    HuggingFaceDownloadFailed(HuggingFaceDownloadFailedParams),
    HuggingFaceModelDoesNotExist(ModelPath),
    HuggingFaceModelUriIsMalformed(ModelPath),
    HuggingFacePermissions(ModelPath),
    LocalModelFileCannotBeChecked(ModelPath),
    ModelCacheIsCorrupted(ModelPath),
    ModelCannotBeLoaded(ModelPath),
    ModelDoesNotExistAtUrl(ModelPath),
    ModelFileDoesNotExist(ModelPath),
    ModelUriIsUnparseable(ModelPath),
    MultimodalProjectionCannotBeLoaded(ModelPath),
    PointerHeadCannotBeLoaded(ModelPath),
    PointerHeadIncompatibleWithModel(PointerHeadIncompatibleWithModelParams),
    SlotCannotStart(SlotCannotStartParams),
    SlotsInsufficientForDecisions(SlotsInsufficientForDecisionsParams),
    UnableToFindChatTemplate(ModelPath),
}
