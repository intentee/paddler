use anyhow::Result;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;

use crate::cluster_handle::ClusterHandle;
use crate::model_card::ModelCard;
use crate::model_card::qwen3_embedding_0_6b::qwen3_embedding_0_6b;
use crate::subprocess_cluster::SubprocessCluster;
use crate::subprocess_cluster_params::SubprocessClusterParams;

pub async fn start_subprocess_cluster_with_qwen3_embedding(
    inference_parameters: InferenceParameters,
    slots_per_agent: i32,
    agent_count: usize,
) -> Result<ClusterHandle> {
    let ModelCard { reference, .. } = qwen3_embedding_0_6b();

    SubprocessCluster::start(SubprocessClusterParams {
        agent_count,
        slots_per_agent,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..SubprocessClusterParams::default()
    })
    .await
}
