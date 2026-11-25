use std::ops::Range;

use crate::embedding_input_tokenized::EmbeddingInputTokenized;

pub struct EmbeddingBatchCapacity {
    pub remaining_sequence_ids: Range<u32>,
    pub remaining_tokens: usize,
}

impl EmbeddingBatchCapacity {
    pub fn admit(
        &mut self,
        EmbeddingInputTokenized { tokens, .. }: &EmbeddingInputTokenized,
    ) -> Option<i32> {
        let remaining_tokens = self.remaining_tokens.checked_sub(tokens.len())?;
        let sequence_id = self.remaining_sequence_ids.next()?;

        self.remaining_tokens = remaining_tokens;

        Some(sequence_id.cast_signed())
    }
}
