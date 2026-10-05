use serde::Deserialize;

use crate::compatibility::openai_service::openai_tool_parameters_schema::OpenAIToolParametersSchema;

#[derive(Deserialize)]
pub struct OpenAIResponsesFunctionTool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: Option<OpenAIToolParametersSchema>,
}
