use serde_json::Value;
use serde_json::json;

#[must_use]
pub fn streamed_greeting_responses_request() -> Value {
    json!({
        "model": "qwen3-test",
        "input": "Say hello.",
        "max_output_tokens": 200,
        "stream": true
    })
}
