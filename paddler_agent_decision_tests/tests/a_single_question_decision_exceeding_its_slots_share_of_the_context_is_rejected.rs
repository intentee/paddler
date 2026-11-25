#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::small_context_decision_cluster_params::small_context_decision_cluster_params;
use paddler_cluster_harness::small_decision_context_size::small_decision_context_size;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;
use paddler_request_params_fixture::repeated_state_decision::repeated_state_decision;

const STATE_REPETITIONS_EXCEEDING_ONE_SLOTS_SHARE: usize = 5;
const TOKENS_REQUIRED_BY_THE_REPEATED_STATE_QUESTION: usize = 159;

#[tokio::test(flavor = "multi_thread")]
async fn a_single_question_decision_exceeding_its_slots_share_of_the_context_is_rejected() {
    let cluster = small_context_decision_cluster_params()
        .start_cluster()
        .await
        .expect("the cluster must start");
    let mut decision = repeated_state_decision(STATE_REPETITIONS_EXCEEDING_ONE_SLOTS_SHARE);

    decision.questions.truncate(1);

    let collected = cluster
        .decide(CancellationToken::new(), &decision)
        .await
        .expect("the oversized decision must be answered with a rejection");

    assert_eq!(collected.answers, Vec::<AnsweredDecisionQuestion>::new());
    assert_eq!(
        collected.terminal_result,
        DecisionResult::RequestExceedsContext(OversizedDecisionDetails {
            context_size: small_decision_context_size().get()
                / u32::from(DECISION_SLOTS_MINIMUM.get()),
            required_tokens: TOKENS_REQUIRED_BY_THE_REPEATED_STATE_QUESTION,
        })
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
