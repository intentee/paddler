use std::num::NonZeroU32;

use anyhow::Result;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::batch_size_within_context::batch_size_within_context;
use crate::qwen3_desired_state::qwen3_desired_state;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_qwen3_and_context_size(
    agents: Vec<AgentConfig>,
    context_size: u32,
) -> Result<Cluster> {
    let context_size = NonZeroU32::try_from(context_size)?;
    let desired_state = qwen3_desired_state();

    start_cluster(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: ModelRuntimeParameters {
                context_size,
                n_batch: batch_size_within_context(context_size)?,
                ..desired_state.model_runtime_parameters
            },
            ..desired_state
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
