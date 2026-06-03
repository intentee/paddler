use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::cluster::Cluster;
use crate::cluster_params::ClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::qwen3_embedding_0_6b::qwen3_embedding_0_6b;
use crate::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use crate::start_cluster::start_cluster;

pub async fn start_embedding_cluster(
    Qwen3EmbeddingClusterParams {
        agents,
        buffered_request_timeout,
        inference_parameters,
        max_buffered_requests,
    }: Qwen3EmbeddingClusterParams,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_embedding_0_6b();

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
