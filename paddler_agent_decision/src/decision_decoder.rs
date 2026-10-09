use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use crate::decision_error::DecisionError;
use crate::decision_question_row::DecisionQuestionRow;

#[expect(
    clippy::cast_possible_wrap,
    reason = "positions stay below the context size the preparer checks, and llama.cpp keeps contexts within i32 positions"
)]
const fn token_position(position: usize) -> i32 {
    position as i32
}

pub struct DecisionDecoder<'scheduler, 'model> {
    pub batch: &'scheduler mut LlamaBatch<'static>,
    pub llama_context: &'scheduler mut LlamaContext<'model>,
    pub n_batch: usize,
}

impl DecisionDecoder<'_, '_> {
    pub fn copy_sequence(
        &mut self,
        source_sequence_id: i32,
        destination_sequence_id: i32,
    ) -> Result<(), DecisionError> {
        self.remove_sequence(destination_sequence_id)?;
        self.llama_context
            .copy_kv_cache_seq(source_sequence_id, destination_sequence_id, None, None)
            .map_err(DecisionError::KvCacheCopyFailed)
    }

    pub fn ingest_state_chunk(
        &mut self,
        sequence_id: i32,
        state: &[LlamaToken],
        next_offset: usize,
    ) -> Result<usize, DecisionError> {
        self.batch.clear();

        for (position, token) in state
            .iter()
            .enumerate()
            .skip(next_offset)
            .take(self.n_batch)
        {
            self.batch
                .add(
                    &SampledToken::Content(*token),
                    token_position(position),
                    &[sequence_id],
                    false,
                )
                .map_err(DecisionError::BatchAssemblyFailed)?;
        }

        self.llama_context
            .decode(self.batch)
            .map_err(DecisionError::DecodeFailed)?;

        Ok(next_offset + self.batch.n_tokens() as usize)
    }

    pub fn decode_row_chunk(
        &mut self,
        sequence_id: i32,
        row: &DecisionQuestionRow,
        next_offset: usize,
        state_length: usize,
        hidden_states: &mut Vec<Vec<f32>>,
    ) -> Result<usize, DecisionError> {
        self.batch.clear();

        let mut output_batch_indices = Vec::new();

        for (batch_index, (row_offset, token)) in (0_i32..).zip(
            row.tokens
                .iter()
                .enumerate()
                .skip(next_offset)
                .take(self.n_batch),
        ) {
            let is_output = row.is_output(row_offset);

            self.batch
                .add(
                    &SampledToken::Content(*token),
                    token_position(state_length + row_offset),
                    &[sequence_id],
                    is_output,
                )
                .map_err(DecisionError::BatchAssemblyFailed)?;

            if is_output {
                output_batch_indices.push(batch_index);
            }
        }

        self.llama_context
            .decode(self.batch)
            .map_err(DecisionError::DecodeFailed)?;

        for batch_index in output_batch_indices {
            hidden_states.push(
                self.llama_context
                    .nextn_embeddings_ith(batch_index)
                    .map_err(DecisionError::HiddenStateUnavailable)?
                    .to_vec(),
            );
        }

        Ok(next_offset + self.batch.n_tokens() as usize)
    }

    pub fn remove_sequence(&mut self, sequence_id: i32) -> Result<(), DecisionError> {
        self.llama_context
            .clear_kv_cache_seq(Some(sequence_id as u32), None, None)
            .map_err(DecisionError::KvCacheRemovalFailed)
    }
}
