use serde_json::Value;
use serde_json::json;

#[must_use]
pub fn noul_system_one_request() -> Value {
    json!({"state": "state", "questions": {"paid": {"type": "noul"}}})
}
