use anyhow::Result;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::cluster_handle::ClusterHandle;
use crate::model_card::ModelCard;
use crate::model_card::qwen3_embedding_0_6b::qwen3_embedding_0_6b;
use crate::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use crate::start_subprocess_cluster::start_subprocess_cluster;
use crate::subprocess_cluster_params::SubprocessClusterParams;

pub async fn start_subprocess_cluster_with_qwen3_embedding(
    Qwen3EmbeddingClusterParams {
        agents,
        buffered_request_timeout,
        inference_parameters,
        max_buffered_requests,
    }: Qwen3EmbeddingClusterParams,
) -> Result<ClusterHandle> {
    let ModelCard { reference, .. } = qwen3_embedding_0_6b();

    start_subprocess_cluster(SubprocessClusterParams {
        agents,
        buffered_request_timeout,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        max_buffered_requests,
        wait_for_slots_ready: true,
        ..SubprocessClusterParams::default()
    })
    .await
}
