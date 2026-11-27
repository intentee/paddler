use crate::decision_prompt_cursor::DecisionPromptCursor;

pub enum DecisionDecodeStep {
    Advanced(DecisionPromptCursor),
    Finished {
        next_cursor: DecisionPromptCursor,
        output_batch_indices: Vec<i32>,
    },
}

impl DecisionDecodeStep {
    #[must_use]
    pub const fn next_cursor(&self) -> DecisionPromptCursor {
        match self {
            Self::Advanced(next_cursor) | Self::Finished { next_cursor, .. } => *next_cursor,
        }
    }
}
