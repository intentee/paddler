use std::num::NonZeroU32;

use anyhow::Result;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
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
    let base_desired_state = qwen3_5_0_8b().into_desired_state();
    let multimodal_projection = if with_mmproj {
        qwen3_5_0_8b_mmproj().into_agent_desired_model()
    } else {
        AgentDesiredModel::None
    };

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            inference_parameters: InferenceParameters {
                context_size,
                n_batch: batch_size_within_context(context_size)?,
                ..base_desired_state.inference_parameters
            },
            multimodal_projection,
            ..base_desired_state
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
