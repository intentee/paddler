use llama_cpp_bindings::eval_multimodal_chunks_params::EvalMultimodalChunksParams;
use llama_cpp_bindings::sampled_token_classifier::SampledTokenClassifier;
use llama_cpp_context::llama_context::LlamaContext;
use llama_cpp_mtmd::mtmd_input_chunks::MtmdInputChunks;

use paddler_agent_runtime::sequence_id::SequenceId;

use crate::image_input::ImageInput;
use crate::media_marked_prompt::MediaMarkedPrompt;
use crate::multimodal_prompt::MultimodalPrompt;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;
use crate::resident_multimodal_context::ResidentMultimodalContext;
use crate::text_generation_error::TextGenerationError;

const ADD_THE_VOCABULARY_SPECIAL_TOKENS: bool = true;

pub enum MultimodalIngestion {
    Supported {
        multimodal_context: ResidentMultimodalContext,
        n_batch: i32,
        sequence_context_size: u32,
    },
    Unsupported,
}

impl MultimodalIngestion {
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
        let Self::Supported {
            multimodal_context,
            n_batch,
            sequence_context_size,
        } = self
        else {
            return Err(TextGenerationError::MultimodalNotSupported);
        };
        let chunks = MtmdInputChunks::tokenize_from_parts(
            &multimodal_context.multimodal_context,
            &MediaMarkedPrompt {
                media_marker,
                prompt_parts,
            }
            .interleave_with(bitmaps)?,
            ADD_THE_VOCABULARY_SPECIAL_TOKENS,
        )
        .map_err(TextGenerationError::MultimodalTokenizationFailed)?;

        require_prompt_fits_sequence_context(chunks.total_tokens(), *sequence_context_size)?;

        token_classifier
            .eval_multimodal_chunks(
                &chunks,
                llama_context,
                EvalMultimodalChunksParams {
                    start_position,
                    seq_id: sequence_id.llama_sequence_id(),
                    n_batch: *n_batch,
                    logits_last: true,
                },
            )
            .map_err(TextGenerationError::for_multimodal_evaluation_error)
    }

    #[must_use]
    pub const fn image_input(&self) -> ImageInput {
        match self {
            Self::Supported { .. } => ImageInput::Supported,
            Self::Unsupported => ImageInput::Unsupported,
        }
    }
}
