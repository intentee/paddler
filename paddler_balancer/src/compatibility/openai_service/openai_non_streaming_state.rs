use llama_cpp_bindings_types::ParsedToolCall;

#[derive(Clone, Default)]
pub struct OpenAINonStreamingState {
    pub content: String,
    pub tool_calls: Vec<ParsedToolCall>,
}
