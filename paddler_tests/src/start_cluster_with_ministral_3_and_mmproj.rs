use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;
use crate::cluster_handle::ClusterHandle;
use crate::current_test_device::current_test_device;
use crate::cluster_params::ClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::ministral_3_14b_reasoning::ministral_3_14b_reasoning;
use crate::model_card::ministral_3_14b_reasoning_mmproj::ministral_3_14b_reasoning_mmproj;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_ministral_3_and_mmproj(
    agents: Vec<AgentConfig>,
) -> Result<ClusterHandle> {
    let device = current_test_device()?;

    device.require_available()?;

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
            inference_parameters: device.inference_parameters_for_full_offload(gpu_layer_count),
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
