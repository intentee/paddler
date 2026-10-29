use paddler_messaging::generated_tool_call::GeneratedToolCall;

#[derive(Debug, Eq, PartialEq)]
pub enum GeneratedOutputPart {
    Content(String),
    Reasoning(String),
    ToolCalls(Vec<GeneratedToolCall>),
}
