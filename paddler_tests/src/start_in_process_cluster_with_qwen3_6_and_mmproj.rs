use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;
use crate::cluster_handle::ClusterHandle;
use crate::current_test_device::current_test_device;
use crate::in_process_cluster_params::InProcessClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::qwen3_6_35b_a3b::qwen3_6_35b_a3b;
use crate::model_card::qwen3_6_35b_a3b_mmproj::qwen3_6_35b_a3b_mmproj;
use crate::start_in_process_cluster::start_in_process_cluster;

pub async fn start_in_process_cluster_with_qwen3_6_and_mmproj(
    agent: AgentConfig,
) -> Result<ClusterHandle> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = qwen3_6_35b_a3b();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = qwen3_6_35b_a3b_mmproj();

    start_in_process_cluster(InProcessClusterParams {
        agent: Some(agent),
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
