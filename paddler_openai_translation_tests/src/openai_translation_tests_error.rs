use serde_json::Error as SerdeJsonError;
use thiserror::Error;

use paddler_openai_translation::generation_failure::GenerationFailure;
use paddler_openai_translation::openai_translation_error::OpenAITranslationError;

#[derive(Debug, Error)]
pub enum OpenAITranslationTestsError {
    #[error("a chunk must serialize")]
    ChunkUnserializable(#[source] SerdeJsonError),

    #[error("a completion must serialize")]
    CompletionUnserializable(#[source] SerdeJsonError),

    #[error("an event must serialize")]
    EventUnserializable(#[source] SerdeJsonError),

    #[error("a finished generation must produce chunks, it failed with {0:?}")]
    GenerationFailed(GenerationFailure),

    #[error("the request must deserialize")]
    RequestUndeserializable(#[source] SerdeJsonError),

    #[error("the request must translate")]
    RequestUntranslatable(#[source] OpenAITranslationError),

    #[error("a response must serialize")]
    ResponseUnserializable(#[source] SerdeJsonError),
}
