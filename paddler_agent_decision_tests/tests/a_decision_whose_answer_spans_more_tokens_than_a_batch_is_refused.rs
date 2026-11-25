#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_cluster_harness::decision_cluster_params::DecisionClusterParams;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::decision_result::DecisionResult;
use paddler_request_params_fixture::sample_decision::sample_decision;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_whose_answer_spans_more_tokens_than_a_batch_is_refused() {
    let cluster = DecisionClusterParams {
        desired_state: DecisionDesiredState {
            n_batch: BatchSize::try_from(u32::from(DECISION_SLOTS_MINIMUM.get()))
                .expect("a batch with one token per decision slot is valid"),
            ..DecisionDesiredState::default()
        },
        ..DecisionClusterParams::default()
    }
    .start_cluster()
    .await
    .expect("the cluster must start");
    let collected = cluster
        .decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("the decision must be answered with a refusal");

    assert_eq!(collected.answers, Vec::<AnsweredDecisionQuestion>::new());
    assert!(matches!(
        collected.terminal_result,
        DecisionResult::RequestRejected(_)
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
