#[derive(Clone, Copy)]
pub struct DecisionPromptPosition {
    pub is_first: bool,
    pub is_last: bool,
}

impl DecisionPromptPosition {
    #[must_use]
    pub const fn among(prompt_index: usize, prompt_count: usize) -> Self {
        Self {
            is_first: prompt_index == 0,
            is_last: prompt_index.saturating_add(1) == prompt_count,
        }
    }
}
