use std::ops::Range;

use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_context::llama_context::LlamaContext;
use llama_cpp_context_memory::context_memory::ContextMemory as _;
use llama_cpp_decision::decision_prompt::DecisionPrompt;
use llama_cpp_model::sampled_token::SampledToken;
use llama_cpp_mtmd::mtmd_input_chunk_view::MtmdInputChunkView;

use paddler_agent_runtime::sequence_id::SequenceId;

use crate::decision_error::DecisionError;
use crate::decision_prompt_cursor::DecisionPromptCursor;
use crate::decision_prompt_outputs_request::DecisionPromptOutputsRequest;
use crate::decoded_text::DecodedText;

pub struct DecisionDecoder<'scheduler, 'model> {
    pub batch: &'scheduler mut LlamaBatch,
    pub llama_context: &'scheduler mut LlamaContext<'model>,
}

impl DecisionDecoder<'_, '_> {
    #[must_use]
    pub const fn batch_tokens(&self) -> usize {
        self.batch.limits().batch as usize
    }

    /// # Errors
    ///
    /// Returns [`DecisionError::KvCacheRemovalFailed`] when the destination sequence cannot be cleared, and [`DecisionError::KvCacheCopyFailed`] when llama.cpp cannot copy the source sequence into it.
    pub fn copy_sequence(
        &self,
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

    ///
    /// # Errors
    ///
    /// Returns [`DecisionError::BatchAssemblyFailed`] when a token does not fit the batch, and [`DecisionError::DecodeFailed`] when llama.cpp rejects the batch.
    pub fn decode_text(
        &mut self,
        prompt: &DecisionPrompt,
        sequence_id: SequenceId,
        cursor: DecisionPromptCursor,
        tokens: Range<usize>,
        outputs: DecisionPromptOutputsRequest,
    ) -> Result<DecodedText, DecisionError> {
        let last_token_index = tokens.end.saturating_sub(1);
        let mut next_cursor = cursor;
        let mut output_batch_indices = Vec::new();

        self.batch.clear();

        for ((batch_index, llama_position), (token_index, token)) in (0_i32..)
            .zip(cursor.llama_position..)
            .zip(prompt.tokens().iter().enumerate().skip(tokens.start))
            .take(tokens.len())
        {
            let is_output = outputs.is_output(token_index, last_token_index);

            self.batch
                .add_with_decision_order(
                    SampledToken::Content(*token),
                    llama_position,
                    &[sequence_id.llama_sequence_id()],
                    is_output,
                    prompt.decision_order(token_index),
                )
                .map_err(DecisionError::BatchAssemblyFailed)?;

            if is_output {
                output_batch_indices.push(batch_index);
            }

            next_cursor = DecisionPromptCursor {
                llama_position: llama_position.saturating_add(1),
                token_index: token_index.saturating_add(1),
            };
        }

        self.llama_context
            .decode(self.batch)
            .map_err(DecisionError::DecodeFailed)?;

        Ok(DecodedText {
            next_cursor,
            output_batch_indices,
        })
    }

    /// # Errors
    ///
    /// Returns [`DecisionError::MediaEvaluationFailed`] when llama.cpp cannot encode or decode the media chunk.
    pub fn evaluate_media(
        &mut self,
        media_chunk: MtmdInputChunkView<'_, '_>,
        sequence_id: SequenceId,
        DecisionPromptCursor {
            llama_position,
            token_index,
        }: DecisionPromptCursor,
    ) -> Result<DecisionPromptCursor, DecisionError> {
        let media_tokens = media_chunk.n_tokens();
        let batch_tokens = self.batch.limits().batch.cast_signed();

        media_chunk
            .eval(
                self.llama_context,
                llama_position,
                sequence_id.llama_sequence_id(),
                batch_tokens,
                false,
            )
            .map(|next_llama_position| DecisionPromptCursor {
                llama_position: next_llama_position,
                token_index: token_index.saturating_add(media_tokens),
            })
            .map_err(DecisionError::MediaEvaluationFailed)
    }

    /// # Errors
    ///
    /// Returns [`DecisionError::KvCacheRemovalFailed`] when llama.cpp cannot remove the sequence from the KV cache.
    pub fn remove_sequence(&self, sequence_id: SequenceId) -> Result<(), DecisionError> {
        sequence_id
            .clear_kv_cache_in(self.llama_context)
            .map_err(DecisionError::KvCacheRemovalFailed)
    }
}
