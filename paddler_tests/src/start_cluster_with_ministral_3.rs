use anyhow::Result;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::ministral_3_cluster_params::Ministral3ClusterParams;
use crate::start_cluster::start_cluster;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::ministral_3_3b_reasoning::ministral_3_3b_reasoning;

pub async fn start_cluster_with_ministral_3(
    Ministral3ClusterParams { agents }: Ministral3ClusterParams,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = ministral_3_3b_reasoning();

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::deterministic()
            },
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
