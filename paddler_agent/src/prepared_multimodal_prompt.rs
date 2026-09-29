use std::sync::Arc;

use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdInputText;

use crate::generation_request_rejection::GenerationRequestRejection;
use crate::multimodal_prompt_ingestion::MultimodalPromptIngestion;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;

pub struct PreparedMultimodalPrompt {
    pub bitmaps: Vec<MtmdBitmap>,
    pub multimodal_context: Arc<MtmdContext>,
    pub n_batch: i32,
    pub text: String,
}

impl PreparedMultimodalPrompt {
    pub fn into_ingestion(
        self,
        sequence_context_size: u32,
    ) -> Result<MultimodalPromptIngestion, GenerationRequestRejection> {
        let bitmap_refs: Vec<&MtmdBitmap> = self.bitmaps.iter().collect();
        let chunks = self
            .multimodal_context
            .tokenize(
                MtmdInputText {
                    text: self.text,
                    add_special: true,
                    parse_special: true,
                },
                &bitmap_refs,
            )
            .map_err(GenerationRequestRejection::MultimodalTokenizationFailed)?;

        require_prompt_fits_sequence_context(chunks.total_tokens(), sequence_context_size)?;

        Ok(MultimodalPromptIngestion {
            chunks,
            multimodal_context: self.multimodal_context,
            n_batch: self.n_batch,
            next_chunk_index: 0,
        })
    }
}
