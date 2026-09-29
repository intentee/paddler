use paddler_messaging::generation_finish::GenerationFinish;

#[must_use]
pub const fn chat_completion_finish_reason(
    finish: GenerationFinish,
    saw_tool_call: bool,
) -> &'static str {
    match finish {
        GenerationFinish::MaxTokens => "length",
        GenerationFinish::EndOfGeneration | GenerationFinish::StopRequested => {
            if saw_tool_call {
                "tool_calls"
            } else {
                "stop"
            }
        }
    }
}
