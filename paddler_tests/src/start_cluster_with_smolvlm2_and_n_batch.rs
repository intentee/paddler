use anyhow::Result;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::smolvlm2_desired_state::smolvlm2_desired_state;
use crate::start_cluster::start_cluster;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

pub async fn start_cluster_with_smolvlm2_and_n_batch(
    agents: Vec<AgentConfig>,
    n_batch: u32,
) -> Result<Cluster> {
    let desired_state = smolvlm2_desired_state();

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            inference_parameters: InferenceParameters {
                n_batch: BatchSize::try_from(n_batch)?,
                ..desired_state.inference_parameters
            },
            ..desired_state
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
