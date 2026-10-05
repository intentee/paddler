use serde_json::Value;
use serde_json::json;

#[must_use]
pub fn output_text_part(text: &str) -> Value {
    json!({
        "type": "output_text",
        "text": text,
        "annotations": [],
        "logprobs": []
    })
}
