use anyhow::Result;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;
use crate::cluster_handle::ClusterHandle;
use crate::current_test_device::current_test_device;
use crate::in_process_cluster_params::InProcessClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::glm_4_7_flash::glm_4_7_flash;
use crate::start_in_process_cluster::start_in_process_cluster;

pub async fn start_in_process_cluster_with_glm_4_7_flash(
    agent: AgentConfig,
) -> Result<ClusterHandle> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = glm_4_7_flash();

    start_in_process_cluster(InProcessClusterParams {
        agent: Some(agent),
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: device.inference_parameters_for_full_offload(gpu_layer_count),
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        },
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await
}
