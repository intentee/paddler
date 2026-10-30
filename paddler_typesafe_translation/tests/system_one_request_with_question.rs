use serde_json::Error as JsonError;
use serde_json::Value;
use serde_json::json;

use paddler_typesafe_translation::system_one_request::SystemOneRequest;

use crate::system_one_request_with_questions::system_one_request_with_questions;

pub fn system_one_request_with_question(question: Value) -> Result<SystemOneRequest, JsonError> {
    system_one_request_with_questions(json!({"question": question}))
}
