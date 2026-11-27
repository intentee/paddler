use serde_json::Value;
use serde_json::json;

use crate::typesafe_request_model::typesafe_request_model;

#[must_use]
pub fn system_one_request(state: &Value, questions: &Value) -> Value {
    json!({"model": typesafe_request_model(), "state": state, "questions": questions})
}
