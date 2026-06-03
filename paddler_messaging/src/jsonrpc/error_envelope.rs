use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorEnvelope<TRequest> {
    pub request_id: String,
    pub error: TRequest,
}
