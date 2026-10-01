use anyhow::Result;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::gemma_3_4b_it::gemma_3_4b_it;
use paddler_test_cluster_harness::model_card::gemma_3_4b_it_mmproj::gemma_3_4b_it_mmproj;

use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_gemma_3_and_mmproj_and_n_batch(
    agents: Vec<AgentConfig>,
    n_batch: u32,
) -> Result<Cluster> {
    let base_desired_state =
        gemma_3_4b_it().into_desired_state_with_multimodal_projection(gemma_3_4b_it_mmproj());

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            inference_parameters: InferenceParameters {
                n_batch: BatchSize::try_from(n_batch)?,
                ..base_desired_state.inference_parameters
            },
            ..base_desired_state
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
