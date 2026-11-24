#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_decision_tests::sample_decision_answers_check::SampleDecisionAnswersCheck;
use paddler_cluster_harness::decision_cluster_params::DecisionClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn kev_answers_every_decision_question_in_order_with_the_expected_input_tokens() {
    SampleDecisionAnswersCheck {
        cluster_params: DecisionClusterParams::default().into_cluster_params(),
    }
    .ensure_for_the_sample_decision()
    .await
    .expect("the sample decision must be answered");
}
