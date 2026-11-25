use llama_cpp_mtmd::mtmd_context::MtmdContext;
use log::info;
use parking_lot::Mutex;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::agent_model_capabilities_error::AgentModelCapabilitiesError;
use crate::multimodal_projection::MultimodalProjection;
use crate::resident_multimodal_context::ResidentMultimodalContext;

pub enum ModelProjector {
    Absent,
    Resident(Mutex<ResidentMultimodalContext>),
}

impl ModelProjector {
    /// # Errors
    ///
    /// Returns [`AgentModelCapabilitiesError::AgentRuntime`] when another multimodal projection occupies the VRAM or the projection file cannot be read, and the errors of [`ResidentMultimodalContext::load`] when llama.cpp cannot load it.
    pub fn load(
        multimodal_projection: &MultimodalProjection,
        loaded_llama_model: &LoadedLlamaModel,
        InferenceRuntimeContext {
            slot_aggregated_status,
            vram_residencies,
            ..
        }: &InferenceRuntimeContext,
    ) -> Result<Self, AgentModelCapabilitiesError> {
        let MultimodalProjection::File(multimodal_projection_path) = multimodal_projection else {
            return Ok(Self::Absent);
        };
        let multimodal_projection_issue_path =
            ModelPath::from(multimodal_projection_path.as_path());
        let vram_occupancy = vram_residencies.multimodal_projection.try_occupy()?;

        match ResidentMultimodalContext::load(
            multimodal_projection_path,
            loaded_llama_model,
            vram_occupancy,
        ) {
            Ok(resident_multimodal_context) => {
                slot_aggregated_status.register_fix(&AgentIssueFix::MultimodalProjectionIsLoaded(
                    multimodal_projection_issue_path,
                ));

                info!(
                    "Multimodal context initialized from: {}",
                    multimodal_projection_path.display()
                );

                Ok(Self::Resident(Mutex::new(resident_multimodal_context)))
            }
            Err(projection_load_error) => {
                slot_aggregated_status.register_pipeline_issue(
                    AgentIssue::MultimodalProjectionCannotBeLoaded(
                        multimodal_projection_issue_path,
                    ),
                );

                Err(projection_load_error)
            }
        }
    }

    pub fn with_multimodal_context<TResult>(
        &self,
        use_multimodal_context: impl FnOnce(Option<&MtmdContext>) -> TResult,
    ) -> TResult {
        match self {
            Self::Absent => use_multimodal_context(None),
            Self::Resident(resident_multimodal_context) => {
                use_multimodal_context(Some(&resident_multimodal_context.lock().multimodal_context))
            }
        }
    }
}
