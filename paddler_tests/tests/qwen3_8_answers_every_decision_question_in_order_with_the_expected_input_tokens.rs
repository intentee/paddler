#![cfg(feature = "tests_that_use_llms")]

use paddler_test_cluster_harness::fixture_path::fixture_path;
use paddler_test_cluster_harness::model_card::qwen3_8_27b::qwen3_8_27b;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::QWEN3_8_27B_SYNTHETIC_POINTER_HEAD_FIXTURE;
use paddler_tests::decision_cluster_params::DecisionClusterParams;

use crate::sample_decision_answers_check::SampleDecisionAnswersCheck;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_8_answers_every_decision_question_in_order_with_the_expected_input_tokens() {
    SampleDecisionAnswersCheck {
        cluster_params: DecisionClusterParams {
            model_card: qwen3_8_27b(),
            pointer_head: fixture_path(QWEN3_8_27B_SYNTHETIC_POINTER_HEAD_FIXTURE),
            ..DecisionClusterParams::default()
        }
        .into_cluster_params()
        .expect("the synthetic pointer head path must be absolute"),
    }
    .assert_for_the_sample_decision()
    .await;
}
