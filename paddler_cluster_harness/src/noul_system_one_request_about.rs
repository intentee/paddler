use serde_json::Value;
use serde_json::json;

use crate::system_one_request::system_one_request;

#[must_use]
pub fn noul_system_one_request_about(state: &Value) -> Value {
    system_one_request(state, &json!({"paid": {"type": "noul"}}))
}
