use llama_cpp_bindings_types::ParsedToolCall;

#[derive(Clone, Default)]
pub struct ResponsesNonStreamingState {
    pub content: String,
    pub reasoning: String,
    pub tool_calls: Vec<ParsedToolCall>,
}
