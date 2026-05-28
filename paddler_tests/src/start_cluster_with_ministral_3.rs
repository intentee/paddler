use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::inference_parameters::InferenceParameters;

use crate::cluster_handle::ClusterHandle;
use crate::cluster_params::ClusterParams;
use crate::make_inference_parameters_deterministic::make_inference_parameters_deterministic;
use crate::ministral_3_cluster_params::Ministral3ClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::ministral_3_14b_reasoning::ministral_3_14b_reasoning;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_ministral_3(
    Ministral3ClusterParams {
        agents,
        deterministic_sampling,
    }: Ministral3ClusterParams,
) -> Result<ClusterHandle> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = ministral_3_14b_reasoning();

    let base_inference_parameters = InferenceParameters {
        n_gpu_layers: gpu_layer_count,
        ..InferenceParameters::default()
    };
    let inference_parameters = if deterministic_sampling {
        make_inference_parameters_deterministic(base_inference_parameters)
    } else {
        base_inference_parameters
    };

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
