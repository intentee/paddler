use serde_json::Value;
use serde_json::json;

use crate::noul_system_one_request_about::noul_system_one_request_about;

#[must_use]
pub fn noul_system_one_request() -> Value {
    noul_system_one_request_about(&json!("state"))
}
