use serde::Deserialize;
use serde::Serialize;

use crate::streamable_result::StreamableResult;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum GeneratedTokenResult {
    ChatTemplateError(String),
    Done,
    Token(String),
}

impl StreamableResult for GeneratedTokenResult {
    fn is_done(&self) -> bool {
        matches!(
            self,
            GeneratedTokenResult::ChatTemplateError(_) | GeneratedTokenResult::Done
        )
    }
}
