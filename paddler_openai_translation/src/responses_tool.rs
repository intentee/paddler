use serde::Deserialize;

use crate::function_definition::FunctionDefinition;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ResponsesTool {
    #[serde(rename = "function")]
    Function(Box<FunctionDefinition>),
}
