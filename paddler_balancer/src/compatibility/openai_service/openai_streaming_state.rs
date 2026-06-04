#[derive(Default)]
pub struct OpenAIStreamingState {
    pub saw_tool_call: bool,
}
