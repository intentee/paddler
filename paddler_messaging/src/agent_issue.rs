use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue_params::ChatTemplateDoesNotCompileParams;
use crate::agent_issue_params::HuggingFaceDownloadLock;
use crate::agent_issue_params::ModelPath;
use crate::agent_issue_params::SlotCannotStartParams;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentIssue {
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
    HuggingFaceModelDoesNotExist(ModelPath),
    HuggingFacePermissions(ModelPath),
    ModelCacheIsCorrupted(ModelPath),
    ModelCannotBeLoaded(ModelPath),
    ModelDoesNotExistAtUrl(ModelPath),
    ModelFileDoesNotExist(ModelPath),
    MultimodalProjectionCannotBeLoaded(ModelPath),
    SlotCannotStart(SlotCannotStartParams),
    UnableToFindChatTemplate(ModelPath),
}
