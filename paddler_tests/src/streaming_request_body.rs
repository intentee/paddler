use serde_json::Value;

pub fn streaming_request_body(body: &Value) -> Value {
    let mut streaming_body = body.clone();

    if let Some(object) = streaming_body.as_object_mut() {
        object.insert("stream".to_owned(), Value::Bool(true));
    }

    streaming_body
}
