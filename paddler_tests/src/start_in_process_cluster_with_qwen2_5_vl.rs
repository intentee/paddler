use anyhow::Result;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::cluster_handle::ClusterHandle;
use crate::current_test_device::current_test_device;
use crate::in_process_cluster::InProcessCluster;
use crate::in_process_cluster_params::InProcessClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::qwen2_5_vl_3b::qwen2_5_vl_3b;
use crate::model_card::qwen2_5_vl_3b_mmproj::qwen2_5_vl_3b_mmproj;

pub async fn start_in_process_cluster_with_qwen2_5_vl(
    slots_per_agent: i32,
) -> Result<ClusterHandle> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = qwen2_5_vl_3b();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = qwen2_5_vl_3b_mmproj();

    InProcessCluster::start(InProcessClusterParams {
        agent_count: 1,
        slots_per_agent,
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: device.inference_parameters_for_full_offload(gpu_layer_count),
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
            use_chat_template_override: false,
        },
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await
}
