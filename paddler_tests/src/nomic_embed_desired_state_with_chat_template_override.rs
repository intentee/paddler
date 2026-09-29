use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

#[must_use]
pub fn nomic_embed_desired_state_with_chat_template_override(
    chat_template: ChatTemplate,
) -> BalancerDesiredState {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = nomic_embed_text_v1_5();

    BalancerDesiredState {
        chat_template_override: Some(chat_template),
        inference_parameters: InferenceParameters {
            n_gpu_layers: gpu_layer_count,
            ..InferenceParameters::default()
        },
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: true,
    }
}
