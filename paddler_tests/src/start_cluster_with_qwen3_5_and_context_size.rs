use std::num::NonZeroU32;

use anyhow::Result;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b_mmproj::qwen3_5_0_8b_mmproj;

use crate::batch_size_within_context::batch_size_within_context;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_qwen3_5_and_context_size(
    agents: Vec<AgentConfig>,
    with_mmproj: bool,
    context_size: u32,
) -> Result<Cluster> {
    let context_size = NonZeroU32::try_from(context_size)?;
    let base_desired_state = if with_mmproj {
        qwen3_5_0_8b().into_desired_state_with_multimodal_projection(qwen3_5_0_8b_mmproj())
    } else {
        qwen3_5_0_8b().into_desired_state()
    };

    start_cluster(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: ModelRuntimeParameters {
                context_size,
                n_batch: batch_size_within_context(context_size)?,
                ..base_desired_state.model_runtime_parameters
            },
            ..base_desired_state
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
