use anyhow::Result;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::gemma_3_4b_it::gemma_3_4b_it;
use paddler_test_cluster_harness::model_card::gemma_3_4b_it_mmproj::gemma_3_4b_it_mmproj;

use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_gemma_3_and_mmproj_and_n_batch(
    agents: Vec<AgentConfig>,
    n_batch: u32,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = gemma_3_4b_it();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = gemma_3_4b_it_mmproj();

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_batch: BatchSize::try_from(n_batch)?,
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
