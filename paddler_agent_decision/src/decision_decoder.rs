use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_runtime::sequence_id::SequenceId;

use crate::decision_error::DecisionError;
use crate::decision_question_row::DecisionQuestionRow;

pub struct DecisionDecoder<'scheduler, 'model> {
    pub batch: &'scheduler mut LlamaBatch<'static>,
    pub llama_context: &'scheduler mut LlamaContext<'model>,
    pub n_batch: usize,
}

impl DecisionDecoder<'_, '_> {
    pub fn copy_sequence(
        &mut self,
        source_sequence_id: SequenceId,
        destination_sequence_id: SequenceId,
    ) -> Result<(), DecisionError> {
        self.remove_sequence(destination_sequence_id)?;
        self.llama_context
            .copy_kv_cache_seq(
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

        for (position, token) in (0_i32..).zip(state).skip(next_offset).take(self.n_batch) {
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

        let mut output_batch_indices = Vec::new();

        let row_chunk = row
            .tokens
            .iter()
            .enumerate()
            .skip(next_offset)
            .take(self.n_batch);
        let row_chunk_positions = (0_i32..).skip(state_length + next_offset);

        for ((batch_index, position), (row_offset, token)) in
            (0_i32..).zip(row_chunk_positions).zip(row_chunk)
        {
            let is_output = row.is_output(row_offset);

            self.batch
                .add(
                    &SampledToken::Content(*token),
                    position,
                    &[sequence_id.llama_sequence_id()],
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

    pub fn remove_sequence(&mut self, sequence_id: SequenceId) -> Result<(), DecisionError> {
        self.llama_context
            .clear_kv_cache_seq(Some(sequence_id.kv_cache_sequence_id()), None, None)
            .map_err(DecisionError::KvCacheRemovalFailed)
    }
}
