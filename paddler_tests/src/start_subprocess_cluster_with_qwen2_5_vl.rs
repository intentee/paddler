use anyhow::Result;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::cluster_handle::ClusterHandle;
use crate::current_test_device::current_test_device;
use crate::model_card::ModelCard;
use crate::model_card::qwen2_5_vl_3b::qwen2_5_vl_3b;
use crate::model_card::qwen2_5_vl_3b_mmproj::qwen2_5_vl_3b_mmproj;
use crate::subprocess_cluster::SubprocessCluster;
use crate::subprocess_cluster_params::SubprocessClusterParams;

pub async fn start_subprocess_cluster_with_qwen2_5_vl(
    slots_per_agent: i32,
    agent_count: usize,
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

    SubprocessCluster::start(SubprocessClusterParams {
        agent_count,
        slots_per_agent,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: device.inference_parameters_for_full_offload(gpu_layer_count),
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..SubprocessClusterParams::default()
    })
    .await
}
