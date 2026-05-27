use anyhow::Result;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;
use crate::cluster_handle::ClusterHandle;
use crate::current_test_device::current_test_device;
use crate::in_process_cluster_params::InProcessClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::smolvlm2_256m::smolvlm2_256m;
use crate::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use crate::start_in_process_cluster::start_in_process_cluster;

pub async fn start_in_process_cluster_with_smolvlm2_and_n_batch(
    agent: AgentConfig,
    n_batch: usize,
) -> Result<ClusterHandle> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = smolvlm2_256m();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = smolvlm2_256m_mmproj();

    let mut inference_parameters = device.inference_parameters_for_full_offload(gpu_layer_count);
    inference_parameters.n_batch = n_batch;

    start_in_process_cluster(InProcessClusterParams {
        agent: Some(agent),
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
            use_chat_template_override: false,
        },
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await
}
