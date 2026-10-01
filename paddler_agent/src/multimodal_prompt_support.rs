use std::sync::Arc;

use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdInputText;

use crate::generation_request_rejection::GenerationRequestRejection;
use crate::multimodal_prompt_ingestion::MultimodalPromptIngestion;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;

pub struct MultimodalPromptSupport {
    pub micro_batch_tokens: u32,
    pub multimodal_context: Arc<MtmdContext>,
    pub n_batch: i32,
    pub sequence_context_size: u32,
}

impl MultimodalPromptSupport {
    pub fn prepare_prompt(
        &self,
        bitmaps: &[MtmdBitmap],
        text: String,
        add_special_tokens: bool,
    ) -> Result<MultimodalPromptIngestion, GenerationRequestRejection> {
        let bitmap_refs: Vec<&MtmdBitmap> = bitmaps.iter().collect();
        let chunks = self
            .multimodal_context
            .tokenize(
                MtmdInputText {
                    text,
                    add_special: add_special_tokens,
                    parse_special: true,
                },
                &bitmap_refs,
            )
            .map_err(GenerationRequestRejection::MultimodalTokenizationFailed)?;

        require_prompt_fits_sequence_context(chunks.total_tokens(), self.sequence_context_size)?;

        chunks
            .fit_to_micro_batch(&self.multimodal_context, self.micro_batch_tokens)
            .map_err(GenerationRequestRejection::for_micro_batch_fit_error)?;

        Ok(MultimodalPromptIngestion {
            chunks,
            multimodal_context: self.multimodal_context.clone(),
            n_batch: self.n_batch,
            next_chunk_index: 0,
        })
    }
}
