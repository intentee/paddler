use serde_json::Value;
use serde_json::json;

use crate::compatibility::openai_service::output_text_part::output_text_part;

#[must_use]
pub fn message_item_done(item_id: &str, text: &str) -> Value {
    json!({
        "id": item_id,
        "type": "message",
        "role": "assistant",
        "status": "completed",
        "content": [output_text_part(text)]
    })
}
