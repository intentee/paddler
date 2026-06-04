use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::agent_config::AgentConfig;
use crate::cluster::Cluster;
use crate::cluster_params::ClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::ministral_3_14b_reasoning::ministral_3_14b_reasoning;
use crate::model_card::ministral_3_14b_reasoning_mmproj::ministral_3_14b_reasoning_mmproj;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_ministral_3_and_mmproj(
    agents: Vec<AgentConfig>,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = ministral_3_14b_reasoning();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = ministral_3_14b_reasoning_mmproj();

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::deterministic()
            },
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
