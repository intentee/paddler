use anyhow::Result;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::start_cluster::start_cluster;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::ministral_3_8b_reasoning::ministral_3_8b_reasoning;
use paddler_test_cluster_harness::model_card::ministral_3_8b_reasoning_mmproj::ministral_3_8b_reasoning_mmproj;

pub async fn start_cluster_with_ministral_3_and_mmproj(
    agents: Vec<AgentConfig>,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = ministral_3_8b_reasoning();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = ministral_3_8b_reasoning_mmproj();

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::deterministic()
            },
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
