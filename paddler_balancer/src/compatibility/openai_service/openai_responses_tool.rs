use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_function_tool::OpenAIResponsesFunctionTool;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIResponsesTool {
    #[serde(rename = "function")]
    Function(Box<OpenAIResponsesFunctionTool>),
    #[serde(other)]
    Unsupported,
}
