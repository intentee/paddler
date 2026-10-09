#![cfg(feature = "tests_that_use_llms")]

use tokio::sync::mpsc;

use paddler_messaging::decision_result::DecisionResult;
use paddler_tests::many_question_decision::many_question_decision;

use crate::serving_decision_agent::ServingDecisionAgent;

const QUESTIONS_OUTLASTING_A_STOP: usize = 96;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_stopped_mid_decision_is_acknowledged() {
    let serving_decision_agent = ServingDecisionAgent::start().await;
    let (decision_stop_tx, decision_stop_rx) = mpsc::unbounded_channel();
    let mut decision_result_rx = serving_decision_agent.forward_decision(
        many_question_decision(QUESTIONS_OUTLASTING_A_STOP),
        decision_stop_rx,
    );

    assert!(matches!(
        decision_result_rx.recv().await,
        Some(DecisionResult::QuestionAnswered(_))
    ));

    decision_stop_tx
        .send(())
        .expect("the stop must reach the decision");

    let mut results_after_the_stop = Vec::new();

    while let Some(decision_result) = decision_result_rx.recv().await {
        results_after_the_stop.push(decision_result);
    }

    assert_eq!(
        results_after_the_stop.last(),
        Some(&DecisionResult::StopRequested)
    );
    assert!(results_after_the_stop.len() < QUESTIONS_OUTLASTING_A_STOP);

    serving_decision_agent.shut_down().await;
}
