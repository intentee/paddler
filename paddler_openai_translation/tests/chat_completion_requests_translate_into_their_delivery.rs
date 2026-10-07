use std::time::Duration;
use std::time::UNIX_EPOCH;

use llama_cpp_bindings_types::TokenUsage;
use serde_json::Value;
use serde_json::from_value;
use serde_json::json;
use serde_json::to_value;

use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_openai_translation::chat_completion_delivery::ChatCompletionDelivery;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_openai_translation::completes_generation::CompletesGeneration as _;
use paddler_openai_translation::generated_output::GeneratedOutput;
use paddler_openai_translation::generation_event::GenerationEvent;

#[cfg(test)]
fn finishing_json_of(request: Value) -> Vec<Value> {
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
        ChatCompletionDelivery::Streamed(mut chat_completion_stream) => chat_completion_stream
            .advance(
                "test-request",
                GenerationEvent::Finished(generation_summary),
            )
            .expect("a finished generation must produce chunks")
            .iter()
            .map(|chat_completion_chunk| {
                to_value(chat_completion_chunk).expect("a chunk must serialize")
            })
            .collect(),
    }
}

#[test]
fn chat_completion_requests_translate_into_their_delivery() {
    let buffered = finishing_json_of(json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "hi"}]
    }));

    assert_eq!(buffered.len(), 1);
    assert_eq!(buffered[0]["object"], "chat.completion");
    assert_eq!(buffered[0]["created"], 1_234);
    assert_eq!(buffered[0]["model"], "test-model");

    let streamed_with_usage = finishing_json_of(json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "hi"}],
        "stream": true,
        "stream_options": {"include_usage": true}
    }));

    assert_eq!(
        streamed_with_usage
            .iter()
            .map(|chunk| (chunk["object"].clone(), chunk.get("usage").is_some()))
            .collect::<Vec<_>>(),
        vec![
            (json!("chat.completion.chunk"), false),
            (json!("chat.completion.chunk"), true)
        ]
    );

    let streamed_without_usage = finishing_json_of(json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "hi"}],
        "stream": true
    }));

    assert_eq!(streamed_without_usage.len(), 1);
}
