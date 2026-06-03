use anyhow::Result;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::model_card::ModelCard;
use crate::model_card::qwen3_0_6b::qwen3_0_6b;
use crate::start_subprocess_cluster::start_subprocess_cluster;

pub async fn start_subprocess_cluster_with_qwen3(
    binary_path: &str,
    agents: Vec<AgentConfig>,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    start_subprocess_cluster(
        binary_path,
        ClusterParams {
            agents,
            desired_state: Some(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters {
                    n_gpu_layers: gpu_layer_count,
                    ..InferenceParameters::default()
                },
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: false,
            }),
            wait_for_slots_ready: true,
            ..ClusterParams::default()
        },
    )
    .await
}
