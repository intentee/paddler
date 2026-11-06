use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::sampled_token::SampledToken;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_runtime::sequence_id::SequenceId;

use crate::decision_error::DecisionError;
use crate::decision_question_row::DecisionQuestionRow;

pub struct DecisionDecoder<'scheduler, 'model> {
    pub batch: &'scheduler mut LlamaBatch,
    pub llama_context: &'scheduler mut LlamaContext<'model>,
}

impl DecisionDecoder<'_, '_> {
    const fn tokens_per_decode(&self) -> usize {
        self.batch.limits().n_batch as usize
    }

    pub fn copy_sequence(
        &mut self,
        source_sequence_id: SequenceId,
        destination_sequence_id: SequenceId,
    ) -> Result<(), DecisionError> {
        self.remove_sequence(destination_sequence_id)?;
        self.llama_context
            .memory_seq_cp(
                source_sequence_id.llama_sequence_id(),
                destination_sequence_id.llama_sequence_id(),
                None,
                None,
            )
            .map_err(DecisionError::KvCacheCopyFailed)
    }

    pub fn ingest_state_chunk(
        &mut self,
        sequence_id: SequenceId,
        state: &[LlamaToken],
        next_offset: usize,
    ) -> Result<usize, DecisionError> {
        self.batch.clear();

        let tokens_per_decode = self.tokens_per_decode();

        for (position, token) in (0_i32..)
            .zip(state)
            .skip(next_offset)
            .take(tokens_per_decode)
        {
            self.batch
                .add(
                    &SampledToken::Content(*token),
                    position,
                    &[sequence_id.llama_sequence_id()],
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
        sequence_id: SequenceId,
        row: &DecisionQuestionRow,
        next_offset: usize,
        state_length: usize,
        hidden_states: &mut Vec<Vec<f32>>,
    ) -> Result<usize, DecisionError> {
        self.batch.clear();

        let row_chunk = row
            .tokens
            .iter()
            .enumerate()
            .skip(next_offset)
            .take(self.tokens_per_decode());
        let row_chunk_positions = (0_i32..).skip(state_length + next_offset);

        for (position, (row_offset, token)) in row_chunk_positions.zip(row_chunk) {
            self.batch
                .add(
                    &SampledToken::Content(*token),
                    position,
                    &[sequence_id.llama_sequence_id()],
                    row.is_output(row_offset),
                )
                .map_err(DecisionError::BatchAssemblyFailed)?;
        }

        self.llama_context.set_embeddings(true);

        let row_hidden_states = self.decode_row_hidden_states();

        self.llama_context.set_embeddings(false);

        hidden_states.extend(row_hidden_states?);

        Ok(next_offset + self.batch.n_tokens() as usize)
    }

    fn decode_row_hidden_states(&mut self) -> Result<Vec<Vec<f32>>, DecisionError> {
        self.llama_context
            .decode(self.batch)
            .map_err(DecisionError::DecodeFailed)?;

        self.batch
            .output_token_indices()
            .iter()
            .map(|batch_index| {
                self.llama_context
                    .embeddings_ith(*batch_index)
                    .map(<[f32]>::to_vec)
                    .map_err(DecisionError::HiddenStateUnavailable)
            })
            .collect()
    }

    pub fn remove_sequence(&mut self, sequence_id: SequenceId) -> Result<(), DecisionError> {
        sequence_id
            .clear_kv_cache_in(self.llama_context)
            .map_err(DecisionError::KvCacheRemovalFailed)
    }
}
