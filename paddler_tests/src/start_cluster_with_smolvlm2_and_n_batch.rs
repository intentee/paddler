use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::inference_parameters::InferenceParameters;

use crate::agent_config::AgentConfig;
use crate::cluster::Cluster;
use crate::cluster_params::ClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::smolvlm2_256m::smolvlm2_256m;
use crate::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_smolvlm2_and_n_batch(
    agents: Vec<AgentConfig>,
    n_batch: usize,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = smolvlm2_256m();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = smolvlm2_256m_mmproj();

    let inference_parameters = InferenceParameters {
        n_gpu_layers: gpu_layer_count,
        n_batch,
        ..InferenceParameters::default()
    };

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
