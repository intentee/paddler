use std::time::Duration;
use std::time::UNIX_EPOCH;

use llama_cpp_bindings_types::TokenUsage;
use serde_json::Value;
use serde_json::from_value;
use serde_json::to_value;

use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_openai_translation::chat_completion_delivery::ChatCompletionDelivery;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_openai_translation::completes_generation::CompletesGeneration as _;
use paddler_openai_translation::generated_output::GeneratedOutput;
use paddler_openai_translation::generation_outcome::GenerationOutcome;

pub fn finishing_json_of(request: Value) -> Vec<Value> {
    let generation_summary = GenerationSummary {
        finish: GenerationFinish::EndOfGeneration,
        usage: TokenUsage::new(),
    };

    match from_value::<ChatCompletionRequest>(request)
        .expect("the request must deserialize")
        .translate(UNIX_EPOCH + Duration::from_secs(1_234))
        .expect("the request must translate")
        .delivery
    {
        ChatCompletionDelivery::Buffered(chat_completion_header) => vec![
            to_value(chat_completion_header.complete(
                "test-request",
                GeneratedOutput::default(),
                &generation_summary,
            ))
            .expect("a completion must serialize"),
        ],
        ChatCompletionDelivery::Streamed(chat_completion_stream) => chat_completion_stream
            .conclude(
                "test-request",
                GenerationOutcome::Finished(generation_summary),
            )
            .expect("a finished generation must produce chunks")
            .iter()
            .map(|chat_completion_chunk| {
                to_value(chat_completion_chunk).expect("a chunk must serialize")
            })
            .collect(),
    }
}
