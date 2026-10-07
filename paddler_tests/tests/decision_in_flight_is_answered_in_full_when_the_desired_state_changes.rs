#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_result::DecisionResult;
use paddler_test_cluster_harness::collect_decision_results::collect_decision_results;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::many_question_decision::many_question_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const QUESTIONS_OUTLASTING_A_STATE_CHANGE: usize = 96;
const CHANGED_CONTEXT_SIZE: u32 = 4096;

#[tokio::test(flavor = "multi_thread")]
async fn decision_in_flight_is_answered_in_full_when_the_desired_state_changes() {
    let cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("the cluster must start");
    let mut decision_stream = cluster
        .client_inference
        .post_decide(
            CancellationToken::new(),
            &many_question_decision(QUESTIONS_OUTLASTING_A_STATE_CHANGE),
        )
        .await
        .expect("the decision must be accepted");

    decision_stream
        .next()
        .await
        .expect("the decision must answer its first question")
        .expect("the first answer must be readable");

    let desired_state = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("the desired state must be readable");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                model_runtime_parameters: ModelRuntimeParameters {
                    context_size: NonZeroU32::new(CHANGED_CONTEXT_SIZE)
                        .expect("the changed context size is not zero"),
                    ..desired_state.model_runtime_parameters
                },
                ..desired_state
            },
        )
        .await
        .expect("the changed desired state must be accepted");

    let collected = collect_decision_results(decision_stream)
        .await
        .expect("the decision in flight must finish");

    assert_eq!(
        collected.answers.len(),
        QUESTIONS_OUTLASTING_A_STATE_CHANGE - 1
    );
    assert!(matches!(collected.terminal_result, DecisionResult::Done(_)));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
