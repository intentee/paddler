use std::fs::File;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::validates::Validates;

pub fn prompt_load_tool(path: &Path) -> Result<Tool<ValidatedParametersSchema>> {
    let file = File::open(path).with_context(|| format!("opening tool file {}", path.display()))?;
    let raw: Tool<RawParametersSchema> = serde_json::from_reader(file)
        .with_context(|| format!("parsing tool file {}", path.display()))?;
    raw.validate()
        .with_context(|| format!("validating tool from {}", path.display()))
}
