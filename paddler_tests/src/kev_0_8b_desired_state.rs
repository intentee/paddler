use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_model_source::model_source::ModelSource;

use crate::kev_0_8b_target_path::kev_0_8b_target_path;

#[must_use]
pub fn kev_0_8b_desired_state() -> BalancerDesiredState {
    BalancerDesiredState {
        decision: DecisionSettings {
            pointer_head: ModelSource::LocalToAgent(
                kev_0_8b_target_path("pointer_head.gguf")
                    .display()
                    .to_string(),
            )
            .into_agent_desired_model(),
        },
        inference_mode: InferenceMode::Decision,
        model: ModelSource::LocalToAgent(kev_0_8b_target_path("model.gguf").display().to_string())
            .into_agent_desired_model(),
        model_runtime_parameters: ModelRuntimeParameters {
            n_gpu_layers: ALL_GPU_LAYERS,
            ..ModelRuntimeParameters::default()
        },
        ..BalancerDesiredState::default()
    }
}
