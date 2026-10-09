#![cfg(feature = "tests_that_use_llms")]

use paddler_test_cluster_harness::model_card::qwen3_8_27b::qwen3_8_27b;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::QWEN3_8_27B_SYNTHETIC_POINTER_HEAD_FIXTURE;
use paddler_tests::decision_cluster_params::DecisionClusterParams;

use crate::every_decision_question_answered_in_order::EveryDecisionQuestionAnsweredInOrder;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_8_answers_every_decision_question_in_order() {
    EveryDecisionQuestionAnsweredInOrder {
        decision_cluster_params: DecisionClusterParams {
            model_card: qwen3_8_27b(),
            pointer_head_fixture: QWEN3_8_27B_SYNTHETIC_POINTER_HEAD_FIXTURE,
            ..DecisionClusterParams::default()
        },
    }
    .assert_for_the_sample_decision()
    .await;
}
