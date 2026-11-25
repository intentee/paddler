#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::constants::SMALL_DECISION_CONTEXT_SIZE;
use paddler_cluster_harness::small_context_decision_cluster_params::small_context_decision_cluster_params;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;
use paddler_request_params_fixture::repeated_state_decision::repeated_state_decision;

const STATE_REPETITIONS_EXCEEDING_THE_CONTEXT: usize = 20;
const TOKENS_REQUIRED_BY_THE_REPEATED_STATE_DECISION: usize = 602;

#[tokio::test(flavor = "multi_thread")]
async fn decision_exceeding_the_context_is_rejected() {
    let cluster = small_context_decision_cluster_params()
        .start_cluster()
        .await
        .expect("the cluster must start");

    let collected = cluster
        .decide(
            CancellationToken::new(),
            &repeated_state_decision(STATE_REPETITIONS_EXCEEDING_THE_CONTEXT),
        )
        .await
        .expect("the oversized decision must be answered with a rejection");

    assert_eq!(collected.answers, Vec::<AnsweredDecisionQuestion>::new());
    assert_eq!(
        collected.terminal_result,
        DecisionResult::RequestExceedsContext(OversizedDecisionDetails {
            context_size: SMALL_DECISION_CONTEXT_SIZE.get(),
            required_tokens: TOKENS_REQUIRED_BY_THE_REPEATED_STATE_DECISION,
        })
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
