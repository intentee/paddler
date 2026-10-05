use serde_json::Value;
use serde_json::json;

#[must_use]
pub fn reasoning_item_done(item_id: &str, text: &str) -> Value {
    json!({
        "type": "reasoning",
        "id": item_id,
        "summary": [],
        "content": [{ "type": "reasoning_text", "text": text }],
        "status": "completed"
    })
}
