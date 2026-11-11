use serde::Deserialize;
use serde::Serialize;

use crate::tool_call_arguments::ToolCallArguments;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedToolCall {
    pub arguments: ToolCallArguments,
    pub id: String,
    pub index: usize,
    pub name: String,
}
