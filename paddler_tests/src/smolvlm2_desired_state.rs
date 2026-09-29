use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_test_cluster_harness::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;

#[must_use]
pub fn smolvlm2_desired_state() -> BalancerDesiredState {
    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = smolvlm2_256m();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = smolvlm2_256m_mmproj();

    BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters {
            n_gpu_layers: gpu_layer_count,
            ..InferenceParameters::deterministic()
        },
        model: AgentDesiredModel::HuggingFace(primary_reference),
        multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
        use_chat_template_override: false,
    }
}
