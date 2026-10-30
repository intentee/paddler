#![cfg(feature = "tests_that_use_llms")]

use paddler_tests::decision_cluster_params::DecisionClusterParams;

use crate::sample_decision_answers_check::SampleDecisionAnswersCheck;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_5_answers_every_decision_question_in_order_with_the_expected_input_tokens() {
    SampleDecisionAnswersCheck {
        cluster_params: DecisionClusterParams::default()
            .into_cluster_params()
            .expect("the synthetic pointer head path must be absolute"),
    }
    .assert_for_the_sample_decision()
    .await;
}
