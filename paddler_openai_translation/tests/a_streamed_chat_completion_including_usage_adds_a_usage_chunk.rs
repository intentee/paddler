use serde_json::json;

use crate::finishing_json_of::finishing_json_of;

#[test]
fn a_streamed_chat_completion_including_usage_adds_a_usage_chunk() {
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
}
