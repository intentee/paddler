use std::sync::Arc;

use llama_cpp_bindings::eval_multimodal_chunks_params::EvalMultimodalChunksParams;
use llama_cpp_bindings::sampled_token_classifier::SampledTokenClassifier;
use llama_cpp_context::llama_context::LlamaContext;
use llama_cpp_mtmd::mtmd_input_chunks::MtmdInputChunks;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_runtime::sequence_id::SequenceId;

use crate::media_marked_prompt::MediaMarkedPrompt;
use crate::multimodal_prompt::MultimodalPrompt;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;
use crate::text_generation_error::TextGenerationError;

const ADD_THE_VOCABULARY_SPECIAL_TOKENS: bool = true;

pub struct MultimodalIngestion {
    pub model_capabilities: Arc<EstablishedModelCapabilities>,
    pub n_batch: i32,
    pub sequence_context_size: u32,
}

impl MultimodalIngestion {
    #[must_use]
    pub fn for_context(
        llama_context: &LlamaContext<'_>,
        model_capabilities: Arc<EstablishedModelCapabilities>,
    ) -> Self {
        Self {
            model_capabilities,
            n_batch: llama_context.n_batch().cast_signed(),
            sequence_context_size: llama_context.n_ctx_seq(),
        }
    }

    /// # Errors
    ///
    /// Returns [`TextGenerationError::MultimodalNotSupported`] when no multimodal projection is loaded, [`TextGenerationError::RenderedMediaMarkersMismatchImages`] when the prompt does not mark every image, [`TextGenerationError::MultimodalTokenizationFailed`] when the multimodal context cannot tokenize it, [`TextGenerationError::PromptExceedsContextSize`] when it does not fit the sequence context, and [`TextGenerationError::MediaExceedsMicroBatch`] or [`TextGenerationError::MultimodalIngestionFailed`] when llama.cpp cannot evaluate its chunks.
    pub fn evaluate(
        &self,
        MultimodalPrompt {
            bitmaps,
            media_marker,
            prompt_parts,
        }: &MultimodalPrompt,
        token_classifier: &mut SampledTokenClassifier<'_>,
        llama_context: &mut LlamaContext<'_>,
        sequence_id: SequenceId,
        start_position: i32,
    ) -> Result<i32, TextGenerationError> {
        self.model_capabilities
            .projector
            .with_multimodal_context(|multimodal_context| {
                let Some(multimodal_context) = multimodal_context else {
                    return Err(TextGenerationError::MultimodalNotSupported);
                };
                let chunks = MtmdInputChunks::tokenize_from_parts(
                    multimodal_context,
                    &MediaMarkedPrompt {
                        media_marker,
                        prompt_parts,
                    }
                    .interleave_with(bitmaps)?,
                    ADD_THE_VOCABULARY_SPECIAL_TOKENS,
                )
                .map_err(TextGenerationError::MultimodalTokenizationFailed)?;

                require_prompt_fits_sequence_context(
                    chunks.total_tokens(),
                    self.sequence_context_size,
                )?;

                token_classifier
                    .eval_multimodal_chunks(
                        &chunks,
                        llama_context,
                        EvalMultimodalChunksParams {
                            start_position,
                            seq_id: sequence_id.llama_sequence_id(),
                            n_batch: self.n_batch,
                            logits_last: true,
                        },
                    )
                    .map_err(TextGenerationError::for_multimodal_evaluation_error)
            })
    }
}
