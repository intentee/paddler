use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;

use crate::cluster_handle::ClusterHandle;
use crate::current_test_device::current_test_device;
use crate::in_process_cluster_params::InProcessClusterParams;
use crate::make_inference_parameters_deterministic::make_inference_parameters_deterministic;
use crate::ministral_3_in_process_cluster_params::Ministral3InProcessClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::ministral_3_14b_reasoning::ministral_3_14b_reasoning;
use crate::start_in_process_cluster::start_in_process_cluster;

pub async fn start_in_process_cluster_with_ministral_3(
    Ministral3InProcessClusterParams {
        agent,
        deterministic_sampling,
    }: Ministral3InProcessClusterParams,
) -> Result<ClusterHandle> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = ministral_3_14b_reasoning();

    let base_inference_parameters = device.inference_parameters_for_full_offload(gpu_layer_count);
    let inference_parameters = if deterministic_sampling {
        make_inference_parameters_deterministic(base_inference_parameters)
    } else {
        base_inference_parameters
    };

    start_in_process_cluster(InProcessClusterParams {
        agent: Some(agent),
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        },
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await
}
