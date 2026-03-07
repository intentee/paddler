use serde::Deserialize;
use serde::Serialize;

use crate::streamable_result::StreamableResult;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum GeneratedTokenResult {
    ChatTemplateError(String),
    Done,
    ImageDecodingFailed(String),
    MultimodalNotSupported(String),
    Token(String),
}

impl StreamableResult for GeneratedTokenResult {
    fn is_done(&self) -> bool {
        matches!(
            self,
            GeneratedTokenResult::ChatTemplateError(_)
                | GeneratedTokenResult::Done
                | GeneratedTokenResult::ImageDecodingFailed(_)
                | GeneratedTokenResult::MultimodalNotSupported(_)
        )
    }
}
