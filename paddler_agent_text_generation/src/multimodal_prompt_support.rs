use std::sync::Arc;

use llama_cpp_bindings::mtmd::FittedMtmdInputChunk;
use llama_cpp_bindings::mtmd::FittedMtmdInputChunks;
use llama_cpp_bindings::mtmd::MtmdBitmap;

use crate::media_marked_prompt::MediaMarkedPrompt;
use crate::model_bos_token::ModelBosToken;
use crate::multimodal_prompt_ingestion::MultimodalPromptIngestion;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;
use crate::resident_multimodal_context::ResidentMultimodalContext;
use crate::text_generation_error::TextGenerationError;

pub struct MultimodalPromptSupport {
    pub micro_batch_tokens: u32,
    pub model_bos_token: ModelBosToken,
    pub multimodal_context: Arc<ResidentMultimodalContext>,
    pub n_batch: i32,
    pub sequence_context_size: u32,
}

impl MultimodalPromptSupport {
    pub fn prepare_prompt(
        &self,
        bitmaps: &[MtmdBitmap],
        media_marked_prompt: &MediaMarkedPrompt,
    ) -> Result<MultimodalPromptIngestion, TextGenerationError> {
        let chunks = self
            .multimodal_context
            .tokenize_from_parts(&media_marked_prompt.interleave_with(bitmaps)?, false)
            .map_err(TextGenerationError::MultimodalTokenizationFailed)?;
        let chunk_tokens = chunks.total_tokens();
        let chunks =
            FittedMtmdInputChunks::fit(chunks, &self.multimodal_context, self.micro_batch_tokens)
                .map_err(TextGenerationError::for_micro_batch_fit_error)?;
        let first_chunk = FittedMtmdInputChunk::at(&chunks, 0);
        let leading_bos_ingestion = self.model_bos_token.leading_bos_ingestion(
            first_chunk
                .as_ref()
                .map_or(&[], FittedMtmdInputChunk::text_tokens),
        );

        require_prompt_fits_sequence_context(
            chunk_tokens + leading_bos_ingestion.token_count(),
            self.sequence_context_size,
        )?;

        Ok(MultimodalPromptIngestion {
            chunks,
            leading_bos_ingestion,
            multimodal_context: self.multimodal_context.clone(),
            n_batch: self.n_batch,
            next_chunk_index: 0,
        })
    }
}
