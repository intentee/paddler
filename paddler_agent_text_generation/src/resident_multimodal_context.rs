use std::ops::Deref;
use std::path::Path;

use llama_cpp_bindings::mtmd::mtmd_context::MtmdContext;
use llama_cpp_bindings::mtmd::mtmd_context_params::MtmdContextParams;

use paddler_agent_runtime::inference_parallelism::InferenceParallelism;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::model_file_prefetch::ModelFilePrefetch;
use paddler_agent_runtime::vram_occupancy::VramOccupancy;

use crate::text_generation_error::TextGenerationError;

#[derive(Debug)]
pub struct ResidentMultimodalContext {
    pub multimodal_context: MtmdContext,
    pub vram_occupancy: VramOccupancy,
}

impl ResidentMultimodalContext {
    /// # Errors
    ///
    /// Returns [`TextGenerationError::MultimodalProjectionLoadFailed`] when llama.cpp cannot load the multimodal projection, and [`TextGenerationError::AgentRuntime`] when it loads but its file cannot be read.
    pub fn load(
        multimodal_projection_path: &Path,
        LoadedLlamaModel {
            inference_parallelism:
                InferenceParallelism {
                    available_parallelism,
                    ..
                },
            model,
            ..
        }: &LoadedLlamaModel,
        vram_occupancy: VramOccupancy,
    ) -> Result<Self, TextGenerationError> {
        ModelFilePrefetch {
            model_path: multimodal_projection_path,
            stream_count: *available_parallelism,
        }
        .alongside(|| {
            #[expect(
                clippy::disallowed_methods,
                reason = "the multimodal projection is loaded here, paired with its VRAM occupancy"
            )]
            let initialized_multimodal_context = MtmdContext::init_from_file(
                &multimodal_projection_path.to_string_lossy(),
                model,
                &MtmdContextParams::default(),
            );

            initialized_multimodal_context.map_err(|source| {
                TextGenerationError::MultimodalProjectionLoadFailed {
                    multimodal_projection_path: multimodal_projection_path.to_path_buf(),
                    source,
                }
            })
        })
        .map(|multimodal_context| Self {
            multimodal_context,
            vram_occupancy,
        })
    }
}

impl Deref for ResidentMultimodalContext {
    type Target = MtmdContext;

    fn deref(&self) -> &MtmdContext {
        &self.multimodal_context
    }
}
