use anyhow::Result;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::collected_decision_results::CollectedDecisionResults;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::constants::DECISION_SLOTS_WITH_A_QUESTION_LANE;

/// # Errors
///
/// Returns an error when the cluster serving `desired_state` cannot start, cannot answer `request`, or cannot shut down.
pub async fn decided_by_a_cluster_serving(
    desired_state: BalancerDesiredState,
    request: &DecisionRequest,
) -> Result<CollectedDecisionResults> {
    let cluster = Cluster::start_in_process(ClusterParams {
        agents: AgentConfig::uniform(1, DECISION_SLOTS_WITH_A_QUESTION_LANE.get()),
        desired_state: ClusterDesiredState::Apply(Box::new(desired_state)),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await?;
    let collected = cluster.decide(CancellationToken::new(), request).await?;

    cluster.shutdown().await?;

    Ok(collected)
}
