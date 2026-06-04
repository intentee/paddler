use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope<TRequest> {
    pub id: String,
    pub request: TRequest,
}
