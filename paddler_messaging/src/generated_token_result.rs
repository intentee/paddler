use serde::Deserialize;
use serde::Serialize;

use llama_cpp_bindings_types::ParsedToolCall;

use crate::generation_summary::GenerationSummary;
use crate::oversized_image_details::OversizedImageDetails;
use crate::raw_tool_call_tokens::RawToolCallTokens;
use crate::streamable_result::StreamableResult;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum GeneratedTokenResult {
    ChatTemplateError(String),
    ContentToken(String),
    DetokenizationFailed(String),
    Done(GenerationSummary),
    GrammarIncompatibleWithThinking(String),
    GrammarInitializationFailed(String),
    GrammarRejectedModelOutput(String),
    GrammarSyntaxError(String),
    ImageDecodingFailed(String),
    ImageExceedsBatchSize(OversizedImageDetails),
    MultimodalNotSupported(String),
    ReasoningToken(String),
    SamplerError(String),
    ToolCallParseFailed(String),
    ToolCallParsed(Vec<ParsedToolCall>),
    ToolCallToken(String),
    ToolCallValidationFailed(Vec<String>),
    ToolSchemaInvalid(String),
    UndeterminableToken(String),
    UnrecognizedToolCallFormat(RawToolCallTokens),
}

impl GeneratedTokenResult {
    #[must_use]
    pub const fn is_token(&self) -> bool {
        matches!(
            self,
            Self::ContentToken(_)
                | Self::ReasoningToken(_)
                | Self::ToolCallToken(_)
                | Self::UndeterminableToken(_)
        )
    }

    #[must_use]
    pub fn token_text(&self) -> Option<&str> {
        match self {
            Self::ContentToken(text)
            | Self::ReasoningToken(text)
            | Self::ToolCallToken(text)
            | Self::UndeterminableToken(text) => Some(text),
            _ => None,
        }
    }

    #[must_use]
    pub const fn is_tool_call_parsed(&self) -> bool {
        matches!(self, Self::ToolCallParsed(_))
    }

    #[must_use]
    pub const fn is_tool_call_failure(&self) -> bool {
        matches!(
            self,
            Self::ToolCallParseFailed(_) | Self::ToolCallValidationFailed(_)
        )
    }
}

impl StreamableResult for GeneratedTokenResult {
    fn is_done(&self) -> bool {
        matches!(
            self,
            Self::ChatTemplateError(_)
                | Self::DetokenizationFailed(_)
                | Self::Done(_)
                | Self::GrammarIncompatibleWithThinking(_)
                | Self::GrammarInitializationFailed(_)
                | Self::GrammarRejectedModelOutput(_)
                | Self::GrammarSyntaxError(_)
                | Self::ImageDecodingFailed(_)
                | Self::ImageExceedsBatchSize(_)
                | Self::MultimodalNotSupported(_)
                | Self::SamplerError(_)
                | Self::ToolSchemaInvalid(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn done_is_done() {
        assert!(GeneratedTokenResult::Done(GenerationSummary::default()).is_done());
    }

    #[test]
    fn chat_template_error_is_done() {
        assert!(GeneratedTokenResult::ChatTemplateError("err".to_owned()).is_done());
    }

    #[test]
    fn detokenization_failed_is_done() {
        assert!(GeneratedTokenResult::DetokenizationFailed("err".to_owned()).is_done());
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
    fn image_exceeds_batch_size_is_done_and_not_classified_as_token() {
        let event = GeneratedTokenResult::ImageExceedsBatchSize(OversizedImageDetails {
            image_tokens: 368,
            n_batch: 100,
        });

        assert!(event.is_done());
        assert!(!event.is_token());
        assert!(event.token_text().is_none());
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
    fn tool_schema_invalid_is_done() {
        assert!(GeneratedTokenResult::ToolSchemaInvalid("invalid schema".to_owned()).is_done());
    }

    #[test]
    fn content_token_is_not_done() {
        assert!(!GeneratedTokenResult::ContentToken("hello".to_owned()).is_done());
    }

    #[test]
    fn reasoning_token_is_not_done() {
        assert!(!GeneratedTokenResult::ReasoningToken("thinking".to_owned()).is_done());
    }

    #[test]
    fn undeterminable_token_is_not_done() {
        assert!(!GeneratedTokenResult::UndeterminableToken("ambiguous".to_owned()).is_done());
    }

    #[test]
    fn tool_call_parsed_is_not_done() {
        let event = GeneratedTokenResult::ToolCallParsed(vec![]);

        assert!(!event.is_done());
        assert!(event.is_tool_call_parsed());
        assert!(!event.is_tool_call_failure());
    }

    #[test]
    fn tool_call_parse_failed_is_failure_but_not_done() {
        let event = GeneratedTokenResult::ToolCallParseFailed("oops".to_owned());

        assert!(!event.is_done());
        assert!(!event.is_tool_call_parsed());
        assert!(event.is_tool_call_failure());
    }

    #[test]
    fn tool_call_validation_failed_is_failure_but_not_done() {
        let event = GeneratedTokenResult::ToolCallValidationFailed(vec!["missing".to_owned()]);

        assert!(!event.is_done());
        assert!(!event.is_tool_call_parsed());
        assert!(event.is_tool_call_failure());
    }

    #[test]
    fn unrecognized_tool_call_format_is_not_done_and_not_classified_as_token() {
        let event = GeneratedTokenResult::UnrecognizedToolCallFormat(RawToolCallTokens {
            text: "raw output".to_owned(),
            ffi_error_message: "parser bailed".to_owned(),
        });

        assert!(!event.is_done());
        assert!(!event.is_token());
        assert!(event.token_text().is_none());
        assert!(!event.is_tool_call_parsed());
        assert!(!event.is_tool_call_failure());
    }
}
