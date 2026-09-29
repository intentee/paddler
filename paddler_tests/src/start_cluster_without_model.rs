use anyhow::Result;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::start_cluster::start_cluster;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

pub async fn start_cluster_without_model(
    agents: Vec<AgentConfig>,
    inference_parameters: InferenceParameters,
) -> Result<Cluster> {
    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::None,
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: false,
        ..ClusterParams::without_request_expiry()
    })
    .await
}
