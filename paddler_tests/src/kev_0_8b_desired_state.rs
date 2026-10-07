use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_inference_settings::BalancerInferenceSettings;
use paddler_messaging::decision_settings::DecisionSettings;

#[must_use]
pub fn kev_0_8b_desired_state() -> BalancerDesiredState {
    BalancerDesiredState {
        inference_settings: BalancerInferenceSettings::Decision(DecisionSettings {
            pointer_head: AgentDesiredModel::LocalToAgent(
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../target/kev/pointer_head.gguf"
                )
                .to_owned(),
            ),
        }),
        model: AgentDesiredModel::LocalToAgent(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../target/kev/model.gguf").to_owned(),
        ),
        model_runtime_parameters: ModelRuntimeParameters {
            n_gpu_layers: ALL_GPU_LAYERS,
            ..ModelRuntimeParameters::default()
        },
    }
}
