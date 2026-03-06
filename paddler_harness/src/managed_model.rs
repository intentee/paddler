use std::sync::Arc;

use anyhow::Result;
use paddler::agent::llamacpp_arbiter::LlamaCppArbiter;
use paddler::agent::llamacpp_arbiter_handle::LlamaCppArbiterHandle;
use paddler::agent::model_metadata_holder::ModelMetadataHolder;
use paddler::agent_desired_state::AgentDesiredState;
use paddler::converts_to_applicable_state::ConvertsToApplicableState;
use paddler::slot_aggregated_status_manager::SlotAggregatedStatusManager;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;

pub struct ManagedModelParams {
    pub inference_parameters: InferenceParameters,
    pub model: HuggingFaceModelReference,
    pub multimodal_projection: Option<HuggingFaceModelReference>,
}

pub struct ManagedModel {
    handle: LlamaCppArbiterHandle,
}

impl ManagedModel {
    pub async fn from_huggingface(params: ManagedModelParams) -> Result<Self> {
        let multimodal_projection = match params.multimodal_projection {
            Some(reference) => AgentDesiredModel::HuggingFace(reference),
            None => AgentDesiredModel::None,
        };

        let desired_state = AgentDesiredState {
            chat_template_override: None,
            inference_parameters: params.inference_parameters,
            model: AgentDesiredModel::HuggingFace(params.model),
            multimodal_projection,
        };

        let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(1));

        let applicable_state = desired_state
            .to_applicable_state(
                slot_aggregated_status_manager
                    .slot_aggregated_status
                    .clone(),
            )
            .await?
            .expect("Failed to convert to applicable state");

        let model_path = applicable_state.model_path.expect("Model path is required");

        let llamacpp_arbiter = LlamaCppArbiter {
            agent_name: Some("managed_test_model".to_string()),
            chat_template_override: None,
            desired_slots_total: 1,
            inference_parameters: applicable_state.inference_parameters,
            multimodal_projection_path: applicable_state.multimodal_projection_path,
            model_metadata_holder: Arc::new(ModelMetadataHolder::new()),
            model_path: model_path.clone(),
            model_path_string: model_path.display().to_string(),
            slot_aggregated_status_manager,
        };

        let handle = llamacpp_arbiter.spawn().await?;

        Ok(Self { handle })
    }

    pub fn handle(&self) -> &LlamaCppArbiterHandle {
        &self.handle
    }

    pub fn shutdown(self) -> Result<()> {
        self.handle.shutdown()
    }
}
