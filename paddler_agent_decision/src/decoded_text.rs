use crate::decision_prompt_cursor::DecisionPromptCursor;

pub struct DecodedText {
    pub next_cursor: DecisionPromptCursor,
    pub output_batch_indices: Vec<i32>,
}
