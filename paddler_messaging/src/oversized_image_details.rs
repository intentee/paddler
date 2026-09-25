use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OversizedImageDetails {
    pub image_tokens: usize,
    pub n_batch: i32,
}
