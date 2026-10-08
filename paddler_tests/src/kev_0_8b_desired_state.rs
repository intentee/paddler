use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::inference_mode::InferenceMode;

#[must_use]
pub fn kev_0_8b_desired_state() -> BalancerDesiredState {
    BalancerDesiredState {
        decision: DecisionSettings {
            pointer_head: AgentDesiredModel::LocalToAgent(
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../target/kev/pointer_head.gguf"
                )
                .to_owned(),
            ),
        },
        inference_mode: InferenceMode::Decision,
        model: AgentDesiredModel::LocalToAgent(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../target/kev/model.gguf").to_owned(),
        ),
        model_runtime_parameters: ModelRuntimeParameters {
            n_gpu_layers: ALL_GPU_LAYERS,
            ..ModelRuntimeParameters::default()
        },
        ..BalancerDesiredState::default()
    }
}
