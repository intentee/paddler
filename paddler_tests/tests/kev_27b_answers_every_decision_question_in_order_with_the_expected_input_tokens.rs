#![cfg(feature = "tests_that_use_llms")]

use paddler_tests::kev_checkpoint::KevCheckpoint;

use crate::sample_decision_answers_check::SampleDecisionAnswersCheck;

#[tokio::test(flavor = "multi_thread")]
async fn kev_27b_answers_every_decision_question_in_order_with_the_expected_input_tokens() {
    SampleDecisionAnswersCheck {
        cluster_params: KevCheckpoint::Kev27b
            .cluster_params()
            .expect("the converted kev-27b artifacts must have absolute paths"),
    }
    .assert_for_the_sample_decision()
    .await;
}
