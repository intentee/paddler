use serde_json::Value;

#[must_use]
pub fn responses_message_text(response: &Value) -> Option<&str> {
    response
        .get("output")
        .and_then(Value::as_array)
        .and_then(|output_items| {
            output_items.iter().find(|output_item| {
                output_item.get("type").and_then(Value::as_str) == Some("message")
            })
        })
        .and_then(|message| message.pointer("/content/0/text"))
        .and_then(Value::as_str)
}
