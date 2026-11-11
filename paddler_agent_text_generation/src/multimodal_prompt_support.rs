use std::sync::Arc;

use llama_cpp_bindings::mtmd::fitted_mtmd_input_chunks::FittedMtmdInputChunks;
use llama_cpp_bindings::mtmd::mtmd_bitmap::MtmdBitmap;

use crate::media_marked_prompt::MediaMarkedPrompt;
use crate::multimodal_prompt_ingestion::MultimodalPromptIngestion;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;
use crate::resident_multimodal_context::ResidentMultimodalContext;
use crate::text_generation_error::TextGenerationError;

const ADD_THE_VOCABULARY_SPECIAL_TOKENS: bool = true;

pub struct MultimodalPromptSupport {
    pub micro_batch_tokens: u32,
    pub multimodal_context: Arc<ResidentMultimodalContext>,
    pub n_batch: i32,
    pub sequence_context_size: u32,
}

impl MultimodalPromptSupport {
    /// # Errors
    ///
    /// Returns [`TextGenerationError::RenderedMediaMarkersMismatchImages`] when the prompt does not mark every image, [`TextGenerationError::MultimodalTokenizationFailed`] when the multimodal context cannot tokenize it, [`TextGenerationError::PromptExceedsContextSize`] when it does not fit the sequence context, and [`TextGenerationError::MediaExceedsMicroBatch`] or [`TextGenerationError::MediaMicroBatchCheckFailed`] when its media chunks cannot be fitted into a micro batch.
    pub fn prepare_prompt(
        &self,
        bitmaps: &[MtmdBitmap],
        media_marked_prompt: &MediaMarkedPrompt,
    ) -> Result<MultimodalPromptIngestion, TextGenerationError> {
        let chunks = self
            .multimodal_context
            .tokenize_from_parts(
                &media_marked_prompt.interleave_with(bitmaps)?,
                ADD_THE_VOCABULARY_SPECIAL_TOKENS,
            )
            .map_err(TextGenerationError::MultimodalTokenizationFailed)?;

        require_prompt_fits_sequence_context(chunks.total_tokens(), self.sequence_context_size)?;

        Ok(MultimodalPromptIngestion {
            chunks: FittedMtmdInputChunks::fit(
                chunks,
                &self.multimodal_context,
                self.micro_batch_tokens,
            )
            .map_err(TextGenerationError::for_micro_batch_fit_error)?,
            multimodal_context: self.multimodal_context.clone(),
            n_batch: self.n_batch,
            next_chunk_index: 0,
        })
    }
}
