use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::start_cluster::start_cluster;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

pub async fn start_embedding_cluster(
    EmbeddingClusterParams {
        agents,
        buffered_request_timeout,
        inference_parameters,
        max_buffered_requests,
    }: EmbeddingClusterParams,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = nomic_embed_text_v1_5();

    let inference_parameters_with_offload = InferenceParameters {
        n_gpu_layers: gpu_layer_count,
        ..inference_parameters
    };

    start_cluster(ClusterParams {
        agents,
        buffered_request_timeout,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: inference_parameters_with_offload,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        max_buffered_requests,
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
