#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionPromptCursor {
    pub llama_position: i32,
    pub token_index: usize,
}
