use serde::Deserialize;
use serde::Serialize;

use crate::streamable_result::StreamableResult;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum GeneratedTokenResult {
    ChatTemplateError(String),
    Done,
    GrammarIncompatibleWithThinking(String),
    GrammarInitializationFailed(String),
    GrammarRejectedModelOutput(String),
    GrammarSyntaxError(String),
    ImageDecodingFailed(String),
    MultimodalNotSupported(String),
    SamplerError(String),
    Token(String),
}

impl StreamableResult for GeneratedTokenResult {
    fn is_done(&self) -> bool {
        matches!(
            self,
            Self::ChatTemplateError(_)
                | Self::Done
                | Self::GrammarIncompatibleWithThinking(_)
                | Self::GrammarInitializationFailed(_)
                | Self::GrammarRejectedModelOutput(_)
                | Self::GrammarSyntaxError(_)
                | Self::ImageDecodingFailed(_)
                | Self::MultimodalNotSupported(_)
                | Self::SamplerError(_)
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
        assert!(GeneratedTokenResult::ChatTemplateError("err".to_owned()).is_done());
    }

    #[test]
    fn grammar_incompatible_with_thinking_is_done() {
        assert!(GeneratedTokenResult::GrammarIncompatibleWithThinking("err".to_owned()).is_done());
    }

    #[test]
    fn grammar_rejected_model_output_is_done() {
        assert!(GeneratedTokenResult::GrammarRejectedModelOutput("err".to_owned()).is_done());
    }

    #[test]
    fn grammar_initialization_failed_is_done() {
        assert!(GeneratedTokenResult::GrammarInitializationFailed("err".to_owned()).is_done());
    }

    #[test]
    fn grammar_syntax_error_is_done() {
        assert!(GeneratedTokenResult::GrammarSyntaxError("err".to_owned()).is_done());
    }

    #[test]
    fn image_decoding_failed_is_done() {
        assert!(GeneratedTokenResult::ImageDecodingFailed("err".to_owned()).is_done());
    }

    #[test]
    fn multimodal_not_supported_is_done() {
        assert!(GeneratedTokenResult::MultimodalNotSupported("err".to_owned()).is_done());
    }

    #[test]
    fn sampler_error_is_done() {
        assert!(GeneratedTokenResult::SamplerError("err".to_owned()).is_done());
    }

    #[test]
    fn token_is_not_done() {
        assert!(!GeneratedTokenResult::Token("hello".to_owned()).is_done());
    }
}
