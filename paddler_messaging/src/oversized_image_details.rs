use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OversizedImageDetails {
    pub image_tokens: u32,
    pub n_batch: u32,
}
