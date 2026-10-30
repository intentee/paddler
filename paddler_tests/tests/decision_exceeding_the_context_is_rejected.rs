#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

use crate::small_context_decision_cluster_params::SMALL_DECISION_CONTEXT_SIZE;
use crate::small_context_decision_cluster_params::small_context_decision_cluster_params;

const STATE_REPETITIONS_EXCEEDING_THE_CONTEXT: usize = 20;
const TOKENS_REQUIRED_BY_THE_REPEATED_STATE_DECISION: usize = 602;

#[tokio::test(flavor = "multi_thread")]
async fn decision_exceeding_the_context_is_rejected() {
    let cluster = start_decision_cluster(small_context_decision_cluster_params())
        .await
        .expect("the cluster must start");

    let collected = cluster
        .decide(
            CancellationToken::new(),
            &RawDecideParams {
                state: sample_decision()
                    .state
                    .repeat(STATE_REPETITIONS_EXCEEDING_THE_CONTEXT),
                ..sample_decision()
            },
        )
        .await
        .expect("the oversized decision must be answered with a rejection");

    assert_eq!(collected.answers, Vec::<DecisionAnswer>::new());
    assert_eq!(
        collected.terminal_result,
        DecisionResult::RequestExceedsContext(OversizedDecisionDetails {
            context_size: SMALL_DECISION_CONTEXT_SIZE,
            required_tokens: TOKENS_REQUIRED_BY_THE_REPEATED_STATE_DECISION,
        })
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
