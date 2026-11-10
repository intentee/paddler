use std::time::UNIX_EPOCH;

use serde_json::json;

use paddler_openai_translation_tests::finishing_json_of::finishing_json_of;

#[test]
fn a_streamed_chat_completion_without_usage_sends_a_single_chunk() {
    let streamed_without_usage = finishing_json_of(
        json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true
        }),
        UNIX_EPOCH,
    )
    .expect("the chat completion must finish into JSON");

    assert_eq!(streamed_without_usage.len(), 1);
}
