use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::in_process_cluster_backend::InProcessClusterBackend;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_model_card::deepseek_r1_distill_llama_8b::deepseek_r1_distill_llama_8b;
use paddler_model_card::model_card::ModelCard;

pub async fn start_cluster_with_deepseek_r1_distill_llama_8b(
    agents: Vec<AgentConfig>,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = deepseek_r1_distill_llama_8b();

    Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents,
            desired_state: DesiredStateInit::set(BalancerDesiredState {
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
