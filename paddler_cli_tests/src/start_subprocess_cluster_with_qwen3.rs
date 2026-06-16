use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::model_card::ModelCard;
use crate::model_card::qwen3_0_6b::qwen3_0_6b;
use crate::subprocess_cluster_backend::SubprocessClusterBackend;

pub async fn start_subprocess_cluster_with_qwen3(
    binary_path: &str,
    agents: Vec<AgentConfig>,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    Cluster::start(
        &SubprocessClusterBackend::new(binary_path),
        ClusterParams {
            agents,
            desired_state: Some(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters {
                    n_gpu_layers: gpu_layer_count,
                    ..InferenceParameters::deterministic()
                },
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: false,
            }),
            wait_for_slots_ready: true,
        },
    )
    .await
}
