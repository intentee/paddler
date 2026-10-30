use serde_json::Value;
use serde_json::json;

use crate::typesafe_request_model::TYPESAFE_REQUEST_MODEL;

#[must_use]
pub fn system_one_request(state: &Value, questions: &Value) -> Value {
    json!({"model": TYPESAFE_REQUEST_MODEL, "state": state, "questions": questions})
}
