use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenAIResponsesFunctionTool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: Option<RawParametersSchema>,
}
