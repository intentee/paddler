#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_test_cluster_harness::cluster::Cluster;

pub async fn answered_decision_probabilities(
    cluster: &Cluster,
    decision: &RawDecideParams,
) -> Vec<Vec<f32>> {
    cluster
        .decide(CancellationToken::new(), decision)
        .await
        .expect("the decision must be answered")
        .answers
        .into_iter()
        .map(|answer| answer.probabilities)
        .collect()
}
