use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope<TResponse> {
    pub generated_by: Option<String>,
    pub request_id: String,
    pub response: TResponse,
}
