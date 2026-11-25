use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::invalid_arguments_json::InvalidArgumentsJson;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ToolCallArguments {
    ValidJson(Value),
    InvalidJson {
        raw: String,
        error: InvalidArgumentsJson,
    },
}
