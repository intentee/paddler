use serde_json::Value;

use paddler_agent_text_generation::tool_call_pipeline::ToolCallPipeline;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_tool_call_validator::tool_call_validation_error::ToolCallValidationError;
use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

/// # Errors
///
/// Returns the [`ToolCallValidationError`] of [`ToolCallValidator::from_tools`] when a tool cannot be validated.
pub fn tool_call_pipeline_for_tools(
    tools: &[Tool<Value>],
) -> Result<ToolCallPipeline, ToolCallValidationError> {
    ToolCallValidator::from_tools(tools).map(ToolCallPipeline::new)
}
