#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;

use paddler_agent_text_generation::tool_call_pipeline::ToolCallPipeline;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

pub fn tool_call_pipeline_for_tools(tools: &[Tool<Value>]) -> ToolCallPipeline {
    ToolCallPipeline::new(ToolCallValidator::from_tools(tools).expect("the tools must validate"))
}
