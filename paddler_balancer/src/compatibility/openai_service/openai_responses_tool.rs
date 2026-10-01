use serde::Deserialize;

use crate::compatibility::openai_service::openai_function_definition::OpenAIFunctionDefinition;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIResponsesTool {
    #[serde(rename = "function")]
    Function(Box<OpenAIFunctionDefinition>),
}
