use std::future::Future;

use anyhow::Result;
use futures_util::TryFutureExt as _;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::collected_decision_results::CollectedDecisionResults;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;

pub fn answered_decision_questions(
    cluster: &Cluster,
    decision: &DecisionRequest,
) -> impl Future<Output = Result<Vec<AnsweredDecisionQuestion>>> + Send + use<> {
    cluster
        .decide(CancellationToken::new(), decision)
        .map_ok(|CollectedDecisionResults { answers, .. }| answers)
}
