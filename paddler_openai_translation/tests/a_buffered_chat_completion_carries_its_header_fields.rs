use serde_json::json;

use crate::finishing_json_of::finishing_json_of;

#[test]
fn a_buffered_chat_completion_carries_its_header_fields() {
    let buffered = finishing_json_of(json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "hi"}]
    }));

    assert_eq!(buffered.len(), 1);
    assert_eq!(buffered[0]["object"], "chat.completion");
    assert_eq!(buffered[0]["created"], 1_234);
    assert_eq!(buffered[0]["model"], "test-model");
}
