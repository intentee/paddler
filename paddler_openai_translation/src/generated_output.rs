use paddler_messaging::generated_tool_call::GeneratedToolCall;

use crate::generated_output_part::GeneratedOutputPart;

#[derive(Default)]
pub struct GeneratedOutput {
    pub content: String,
    pub reasoning: String,
    pub tool_calls: Vec<GeneratedToolCall>,
}

impl GeneratedOutput {
    pub fn append(&mut self, generated_output_part: GeneratedOutputPart) {
        match generated_output_part {
            GeneratedOutputPart::Content(text) => self.content.push_str(&text),
            GeneratedOutputPart::Reasoning(text) => self.reasoning.push_str(&text),
            GeneratedOutputPart::ToolCalls(parsed_calls) => self.tool_calls.extend(parsed_calls),
        }
    }
}
