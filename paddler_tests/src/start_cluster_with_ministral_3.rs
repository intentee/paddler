use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::ministral_3_cluster_params::Ministral3ClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::ministral_3_14b_reasoning::ministral_3_14b_reasoning;
use crate::start_cluster::start_cluster;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

pub async fn start_cluster_with_ministral_3(
    Ministral3ClusterParams {
        agents,
        deterministic_sampling,
    }: Ministral3ClusterParams,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = ministral_3_14b_reasoning();

    let inference_parameters = if deterministic_sampling {
        InferenceParameters {
            n_gpu_layers: gpu_layer_count,
            ..InferenceParameters::deterministic()
        }
    } else {
        InferenceParameters {
            n_gpu_layers: gpu_layer_count,
            ..InferenceParameters::default()
        }
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
