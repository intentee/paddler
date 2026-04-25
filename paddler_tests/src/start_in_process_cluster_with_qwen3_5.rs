use anyhow::Result;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::cluster_handle::ClusterHandle;
use crate::current_test_device::current_test_device;
use crate::in_process_cluster::InProcessCluster;
use crate::in_process_cluster_params::InProcessClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use crate::model_card::qwen3_5_0_8b_mmproj::qwen3_5_0_8b_mmproj;

pub async fn start_in_process_cluster_with_qwen3_5(
    slots_per_agent: i32,
    with_mmproj: bool,
) -> Result<ClusterHandle> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = qwen3_5_0_8b();

    let multimodal_projection = if with_mmproj {
        let ModelCard {
            reference: mmproj_reference,
            ..
        } = qwen3_5_0_8b_mmproj();

        AgentDesiredModel::HuggingFace(mmproj_reference)
    } else {
        AgentDesiredModel::None
    };

    InProcessCluster::start(InProcessClusterParams {
        agent_count: 1,
        slots_per_agent,
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: device.inference_parameters_for_full_offload(gpu_layer_count),
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection,
            use_chat_template_override: false,
        },
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await
}
