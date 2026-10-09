#![cfg(feature = "tests_that_use_llms")]

use tokio::sync::mpsc;

use paddler_messaging::decision_result::DecisionResult;
use paddler_tests::sample_decision::sample_decision;

use crate::serving_decision_agent::ServingDecisionAgent;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_stopped_before_admission_is_acknowledged() {
    let serving_decision_agent = ServingDecisionAgent::start().await;
    let (decision_stop_tx, decision_stop_rx) = mpsc::unbounded_channel();

    decision_stop_tx
        .send(())
        .expect("the stop must reach the decision");

    let mut decision_result_rx =
        serving_decision_agent.forward_decision(sample_decision(), decision_stop_rx);

    assert_eq!(
        decision_result_rx.recv().await,
        Some(DecisionResult::StopRequested)
    );
    assert_eq!(decision_result_rx.recv().await, None);

    serving_decision_agent.shut_down().await;
}
