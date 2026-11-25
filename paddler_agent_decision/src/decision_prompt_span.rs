use llama_cpp_decision::decision_prompt::DecisionPrompt;
use llama_cpp_mtmd::mtmd_context::MtmdContext;
use llama_cpp_mtmd::mtmd_input_chunk_view::MtmdInputChunkView;

use paddler_agent_runtime::sequence_id::SequenceId;

use crate::decision_decode_step::DecisionDecodeStep;
use crate::decision_decoder::DecisionDecoder;
use crate::decision_error::DecisionError;
use crate::decision_prompt_cursor::DecisionPromptCursor;
use crate::decision_prompt_outputs_request::DecisionPromptOutputsRequest;
use crate::decoded_text::DecodedText;

pub struct DecisionPromptSpan<'prompt, 'projector> {
    pub end: usize,
    pub final_outputs: DecisionPromptOutputsRequest,
    pub projector: Option<&'projector MtmdContext>,
    pub prompt: &'prompt DecisionPrompt,
    pub sequence_id: SequenceId,
}

impl DecisionPromptSpan<'_, '_> {
    /// Decodes the next media chunk or text chunk of the span. The last text chunk ends the span and holds the outputs the span asks for, so every token from where the answer is read to the end of the span is decoded in one batch.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`DecisionDecoder::evaluate_media`] and [`DecisionDecoder::decode_text`].
    pub fn decode_step(
        &self,
        decoder: &mut DecisionDecoder<'_, '_>,
        cursor: DecisionPromptCursor,
    ) -> Result<DecisionDecodeStep, DecisionError> {
        if let Some(media_chunk) = self.media_chunk_at(cursor.token_index) {
            return decoder
                .evaluate_media(media_chunk, self.sequence_id, cursor)
                .map(DecisionDecodeStep::Advanced);
        }

        let text_end = self.text_end_after(cursor.token_index);
        let batch_tokens = decoder.batch_tokens();

        if text_end == self.end && self.end.saturating_sub(cursor.token_index) <= batch_tokens {
            let DecodedText {
                next_cursor,
                output_batch_indices,
            } = decoder.decode_text(
                self.prompt,
                self.sequence_id,
                cursor,
                cursor.token_index..self.end,
                self.final_outputs,
            )?;

            return Ok(DecisionDecodeStep::Finished {
                next_cursor,
                output_batch_indices,
            });
        }

        let chunk_end = if text_end == self.end {
            self.end.saturating_sub(batch_tokens)
        } else {
            text_end
        }
        .min(cursor.token_index.saturating_add(batch_tokens));

        decoder
            .decode_text(
                self.prompt,
                self.sequence_id,
                cursor,
                cursor.token_index..chunk_end,
                DecisionPromptOutputsRequest::None,
            )
            .map(|DecodedText { next_cursor, .. }| DecisionDecodeStep::Advanced(next_cursor))
    }

    fn media_chunk_at(&self, token_index: usize) -> Option<MtmdInputChunkView<'_, '_>> {
        self.projector
            .and_then(|projector| self.prompt.media_chunk(token_index, projector))
    }

    fn text_end_after(&self, token_index: usize) -> usize {
        self.projector.map_or(self.end, |projector| {
            (token_index.saturating_add(1)..self.end)
                .find(|media_start| self.prompt.media_chunk(*media_start, projector).is_some())
                .unwrap_or(self.end)
        })
    }
}
