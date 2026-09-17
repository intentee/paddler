use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawToolCallTokens {
    pub ffi_error_message: String,
    pub synthetic_render_with_tools: String,
    pub synthetic_render_without_tools: String,
    pub text: String,
}
