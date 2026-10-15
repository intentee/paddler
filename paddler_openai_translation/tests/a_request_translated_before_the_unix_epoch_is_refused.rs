use std::error::Error as _;
use std::time::Duration;
use std::time::SystemTimeError;
use std::time::UNIX_EPOCH;

use serde_json::from_value;
use serde_json::json;

use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_openai_translation::openai_translation_error::OpenAITranslationError;
use paddler_openai_translation::responses_request::ResponsesRequest;

fn clock_error_duration(translation_error: &OpenAITranslationError) -> Option<Duration> {
    translation_error
        .source()
        .and_then(|source| source.downcast_ref::<SystemTimeError>())
        .map(SystemTimeError::duration)
}

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
    let clock_before_unix_epoch = OpenAITranslationError::ClockBeforeUnixEpoch(
        UNIX_EPOCH
            .duration_since(UNIX_EPOCH + Duration::from_secs(1))
            .expect_err("the Unix epoch must precede the second after it"),
    );

    let chat_completion_error = chat_completion_request.translate(before_unix_epoch).err();
    let responses_error = responses_request
        .translate(before_unix_epoch, &SamplingParameters::default())
        .err();

    assert_eq!(
        chat_completion_error.as_ref().map(ToString::to_string),
        Some(clock_before_unix_epoch.to_string())
    );
    assert_eq!(
        chat_completion_error
            .as_ref()
            .and_then(clock_error_duration),
        Some(Duration::from_secs(1))
    );
    assert_eq!(
        responses_error.as_ref().map(ToString::to_string),
        Some(clock_before_unix_epoch.to_string())
    );
    assert_eq!(
        responses_error.as_ref().and_then(clock_error_duration),
        Some(Duration::from_secs(1))
    );
}
