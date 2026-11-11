use std::mem::discriminant;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::TryFutureExt as _;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::collected_decision_results::CollectedDecisionResults;

use crate::decision_cluster_params::DecisionClusterParams;
use crate::decision_done_discriminant::decision_done_discriminant;
use crate::sample_decision::sample_decision;

async fn answer_the_sample_decision(cluster: Cluster) -> Result<Cluster> {
    cluster
        .decide(CancellationToken::new(), &sample_decision())
        .await
        .and_then(|CollectedDecisionResults { terminal_result, .. }| {
            (discriminant(&terminal_result) == decision_done_discriminant())
                .then_some(cluster)
                .context(format!(
                    "the decision cluster must finish its sample decision, but it ended with {terminal_result:?}"
                ))
        })
}

/// # Errors
///
/// Returns an error when the decision cluster cannot start, when it does not answer the sample decision, and when that decision does not finish.
pub async fn start_decision_cluster_that_answered_a_decision() -> Result<Cluster> {
    DecisionClusterParams::default()
        .start_cluster()
        .and_then(answer_the_sample_decision)
        .await
}
