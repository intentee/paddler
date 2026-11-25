use std::time::Duration;
use std::time::UNIX_EPOCH;

use serde_json::json;

use paddler_openai_translation_tests::finishing_json_of::finishing_json_of;

#[test]
fn a_buffered_chat_completion_carries_its_header_fields() {
    let buffered = finishing_json_of(
        json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}]
        }),
        UNIX_EPOCH
            .checked_add(Duration::from_secs(1_234))
            .expect("1234 seconds after the Unix epoch must be representable"),
    )
    .expect("the chat completion must finish into JSON");
    let completion = buffered
        .first()
        .expect("the buffered chat completion must finish into one completion");

    assert_eq!(buffered.len(), 1);
    assert_eq!(completion.get("object"), Some(&json!("chat.completion")));
    assert_eq!(completion.get("created"), Some(&json!(1_234)));
    assert_eq!(completion.get("model"), Some(&json!("test-model")));
}
