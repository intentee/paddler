use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_function_tool::OpenAIResponsesFunctionTool;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIResponsesTool {
    // Boxed because the function-tool payload is far larger than the empty `Unsupported` variant.
    #[serde(rename = "function")]
    Function(Box<OpenAIResponsesFunctionTool>),
    #[serde(other)]
    Unsupported,
}
