use llama_cpp_bindings_types::ParsedToolCall;

#[derive(Debug, Eq, PartialEq)]
pub enum GeneratedOutputPart {
    Content(String),
    Reasoning(String),
    ToolCalls(Vec<ParsedToolCall>),
}
