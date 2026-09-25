use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawToolCallTokens {
    pub text: String,
    pub ffi_error_message: String,
}
