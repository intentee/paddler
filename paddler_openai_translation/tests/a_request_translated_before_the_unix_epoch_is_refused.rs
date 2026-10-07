use std::time::Duration;
use std::time::UNIX_EPOCH;

use serde_json::from_value;
use serde_json::json;

use paddler_messaging::agent_inference_settings::AgentInferenceSettings;
use paddler_messaging::agent_text_generation_settings::AgentTextGenerationSettings;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_openai_translation::openai_translation_error::OpenAITranslationError;
use paddler_openai_translation::responses_request::ResponsesRequest;

#[test]
fn a_request_translated_before_the_unix_epoch_is_refused() {
    let before_unix_epoch = UNIX_EPOCH - Duration::from_secs(1);
    let chat_completion_request: ChatCompletionRequest = from_value(json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "hi"}]
    }))
    .expect("the chat completion request must deserialize");
    let responses_request: ResponsesRequest =
        from_value(json!({"model": "test-model", "input": "hi"}))
            .expect("the responses request must deserialize");

    assert!(matches!(
        chat_completion_request.translate(before_unix_epoch),
        Err(OpenAITranslationError::ClockBeforeUnixEpoch(clock_error))
            if clock_error.duration() == Duration::from_secs(1)
    ));
    assert!(matches!(
        responses_request.translate(
            before_unix_epoch,
            &AgentInferenceSettings::TextGeneration(AgentTextGenerationSettings::default())
        ),
        Err(OpenAITranslationError::ClockBeforeUnixEpoch(clock_error))
            if clock_error.duration() == Duration::from_secs(1)
    ));
}
