use std::fmt;

use paddler_types::oversized_image_details::OversizedImageDetails;

#[derive(Debug)]
pub enum StopReason {
    Completed,
    ChatTemplateError(String),
    GrammarIncompatibleWithThinking(String),
    GrammarInitializationFailed(String),
    GrammarRejectedModelOutput(String),
    GrammarSyntaxError(String),
    ImageDecodingFailed(String),
    ImageExceedsBatchSize(OversizedImageDetails),
    InferenceError { code: i32, description: String },
    MultimodalNotSupported(String),
    SamplerError(String),
    Timeout,
    TooManyBufferedRequests,
    ToolCallParseFailed(String),
    ToolCallValidationFailed(Vec<String>),
    ToolSchemaInvalid(String),
    WireStreamError(String),
}

impl fmt::Display for StopReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Completed => formatter.write_str("completed"),
            Self::ChatTemplateError(detail) => {
                write!(formatter, "chat template error: {detail}")
            }
            Self::GrammarIncompatibleWithThinking(detail) => {
                write!(formatter, "grammar incompatible with thinking: {detail}")
            }
            Self::GrammarInitializationFailed(detail) => {
                write!(formatter, "grammar initialization failed: {detail}")
            }
            Self::GrammarRejectedModelOutput(detail) => {
                write!(formatter, "grammar rejected model output: {detail}")
            }
            Self::GrammarSyntaxError(detail) => {
                write!(formatter, "grammar syntax error: {detail}")
            }
            Self::ImageDecodingFailed(detail) => {
                write!(formatter, "image decoding failed: {detail}")
            }
            Self::ImageExceedsBatchSize(details) => {
                write!(
                    formatter,
                    "image required {} tokens but agent n_batch is {}",
                    details.image_tokens, details.n_batch,
                )
            }
            Self::InferenceError { code, description } => {
                write!(formatter, "inference error {code}: {description}")
            }
            Self::MultimodalNotSupported(detail) => {
                write!(formatter, "multimodal input not supported: {detail}")
            }
            Self::SamplerError(detail) => write!(formatter, "sampler error: {detail}"),
            Self::Timeout => formatter.write_str("balancer timed out the request"),
            Self::TooManyBufferedRequests => {
                formatter.write_str("balancer rejected the request: queue is full")
            }
            Self::ToolCallParseFailed(detail) => {
                write!(formatter, "tool-call parse failed: {detail}")
            }
            Self::ToolCallValidationFailed(field_errors) => {
                write!(
                    formatter,
                    "tool-call validation failed: {}",
                    field_errors.join("; ")
                )
            }
            Self::ToolSchemaInvalid(detail) => {
                write!(formatter, "tool schema invalid: {detail}")
            }
            Self::WireStreamError(detail) => {
                write!(formatter, "wire stream error: {detail}")
            }
        }
    }
}
