use std::future::Future;

use anyhow::Result;
use futures_util::TryFutureExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::collected_decision_results::CollectedDecisionResults;

pub fn answered_decision_probabilities(
    cluster: &Cluster,
    decision: &RawDecideParams,
) -> impl Future<Output = Result<Vec<Vec<f32>>>> + Send + use<> {
    cluster.decide(CancellationToken::new(), decision).map_ok(
        |CollectedDecisionResults { answers, .. }| {
            answers
                .into_iter()
                .map(|DecisionAnswer { probabilities, .. }| probabilities)
                .collect()
        },
    )
}
