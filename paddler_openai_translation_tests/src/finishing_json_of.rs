use std::time::SystemTime;

use serde_json::Value;
use serde_json::from_value;
use serde_json::to_value;

use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::token_usage::TokenUsage;
use paddler_openai_translation::chat_completion_delivery::ChatCompletionDelivery;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_openai_translation::completes_generation::CompletesGeneration as _;
use paddler_openai_translation::generated_output::GeneratedOutput;
use paddler_openai_translation::generation_outcome::GenerationOutcome;
use paddler_openai_translation::translated_chat_completion_request::TranslatedChatCompletionRequest;

use crate::openai_translation_tests_error::OpenAITranslationTestsError;

fn finishing_json_of_delivery(
    delivery: ChatCompletionDelivery,
) -> Result<Vec<Value>, OpenAITranslationTestsError> {
    let generation_summary = GenerationSummary {
        finish: GenerationFinish::EndOfGeneration,
        usage: TokenUsage::default(),
    };

    match delivery {
        ChatCompletionDelivery::Buffered(chat_completion_header) => {
            to_value(chat_completion_header.complete(
                "test-request",
                GeneratedOutput::default(),
                &generation_summary,
            ))
            .map(|completion| vec![completion])
            .map_err(OpenAITranslationTestsError::CompletionUnserializable)
        }
        ChatCompletionDelivery::Streamed(chat_completion_stream) => chat_completion_stream
            .conclude(
                "test-request",
                GenerationOutcome::Finished(generation_summary),
            )
            .map_err(OpenAITranslationTestsError::GenerationFailed)
            .and_then(|chat_completion_chunks| {
                chat_completion_chunks
                    .iter()
                    .map(|chat_completion_chunk| {
                        to_value(chat_completion_chunk)
                            .map_err(OpenAITranslationTestsError::ChunkUnserializable)
                    })
                    .collect()
            }),
    }
}

/// # Errors
///
/// Returns [`OpenAITranslationTestsError::RequestUndeserializable`] when `request` is not a chat completion request, [`OpenAITranslationTestsError::RequestUntranslatable`] when the request cannot be translated at `now`, [`OpenAITranslationTestsError::GenerationFailed`] when the finished generation produces no chunks, and [`OpenAITranslationTestsError::CompletionUnserializable`] or [`OpenAITranslationTestsError::ChunkUnserializable`] when the completion or a chunk cannot be serialized.
pub fn finishing_json_of(
    request: Value,
    now: SystemTime,
) -> Result<Vec<Value>, OpenAITranslationTestsError> {
    from_value::<ChatCompletionRequest>(request)
        .map_err(OpenAITranslationTestsError::RequestUndeserializable)
        .and_then(|chat_completion_request| {
            chat_completion_request
                .translate(now)
                .map_err(OpenAITranslationTestsError::RequestUntranslatable)
        })
        .and_then(|TranslatedChatCompletionRequest { delivery, .. }| {
            finishing_json_of_delivery(delivery)
        })
}
