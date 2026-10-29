#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::ChatMessageParser;
use llama_cpp_bindings::model::LlamaModel;
use serde_json::Value;

use paddler_agent_text_generation::chat_template_tools::ChatTemplateTools;
use paddler_agent_text_generation::tool_call_pipeline::ToolCallPipeline;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

pub fn tool_call_pipeline_for_tools(model: &LlamaModel, tools: &[Tool<Value>]) -> ToolCallPipeline {
    ToolCallPipeline::new(
        ChatMessageParser::new(model, &ChatTemplateTools::new(tools).json)
            .expect("the chat message parser must be created"),
        ToolCallValidator::from_tools(tools).expect("the tools must validate"),
    )
}
