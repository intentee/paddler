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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn done_is_done() {
        assert!(GeneratedTokenResult::Done.is_done());
    }

    #[test]
    fn chat_template_error_is_done() {
        assert!(GeneratedTokenResult::ChatTemplateError("err".to_string()).is_done());
    }

    #[test]
    fn image_decoding_failed_is_done() {
        assert!(GeneratedTokenResult::ImageDecodingFailed("err".to_string()).is_done());
    }

    #[test]
    fn multimodal_not_supported_is_done() {
        assert!(GeneratedTokenResult::MultimodalNotSupported("err".to_string()).is_done());
    }

    #[test]
    fn token_is_not_done() {
        assert!(!GeneratedTokenResult::Token("hello".to_string()).is_done());
    }
}
