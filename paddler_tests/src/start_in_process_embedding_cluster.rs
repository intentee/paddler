use anyhow::Result;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;

use crate::cluster_handle::ClusterHandle;
use crate::in_process_cluster::InProcessCluster;
use crate::in_process_cluster_params::InProcessClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::qwen3_embedding_0_6b::qwen3_embedding_0_6b;

pub async fn start_in_process_embedding_cluster(
    inference_parameters: InferenceParameters,
    slots_per_agent: i32,
) -> Result<ClusterHandle> {
    let ModelCard { reference, .. } = qwen3_embedding_0_6b();

    InProcessCluster::start(InProcessClusterParams {
        slots_per_agent,
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        },
        ..InProcessClusterParams::default()
    })
    .await
}
