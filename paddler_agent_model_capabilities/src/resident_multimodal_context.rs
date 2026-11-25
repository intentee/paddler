use std::path::Path;

use llama_cpp_decision::decision_model::DecisionModel;
use llama_cpp_mtmd::mtmd_context::MtmdContext;
use llama_cpp_mtmd::mtmd_context_params::MtmdContextParams;

use paddler_agent_runtime::inference_parallelism::InferenceParallelism;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::model_file_prefetch::ModelFilePrefetch;
use paddler_agent_runtime::vram_occupancy::VramOccupancy;

use crate::agent_model_capabilities_error::AgentModelCapabilitiesError;

#[derive(Debug)]
pub struct ResidentMultimodalContext {
    pub multimodal_context: MtmdContext,
    pub vram_occupancy: VramOccupancy,
}

impl ResidentMultimodalContext {
    /// # Errors
    ///
    /// Returns [`AgentModelCapabilitiesError::MultimodalProjectionLoadFailed`] when llama.cpp cannot load the multimodal projection, and [`AgentModelCapabilitiesError::AgentRuntime`] when it loads but its file cannot be read.
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
    ) -> Result<Self, AgentModelCapabilitiesError> {
        let multimodal_context_params = MtmdContextParams {
            media_marker: DecisionModel::media_marker().to_owned(),
            ..MtmdContextParams::default()
        };

        ModelFilePrefetch {
            model_path: multimodal_projection_path,
            stream_count: *available_parallelism,
        }
        .alongside(|| {
            MtmdContext::init_from_file(
                &multimodal_projection_path.to_string_lossy(),
                model.llama_model.clone(),
                &multimodal_context_params,
            )
            .map_err(|source| {
                AgentModelCapabilitiesError::MultimodalProjectionLoadFailed {
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
