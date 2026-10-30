use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub enum HuggingFaceDownloadFailure {
    CacheIoFailed,
    HubConnectionFailed,
    HubRespondedWithStatus { status: u16 },
    HubResponseMalformed,
    UnexpectedHubClientFailure,
    XetTransferFailed,
}
