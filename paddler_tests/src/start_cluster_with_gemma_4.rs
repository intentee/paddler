use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::agent_config::AgentConfig;
use crate::cluster::Cluster;
use crate::cluster_params::ClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::gemma_4_e4b_it::gemma_4_e4b_it;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_gemma_4(agents: Vec<AgentConfig>) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = gemma_4_e4b_it();

    start_cluster(ClusterParams {
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
        ..ClusterParams::default()
    })
    .await
}
