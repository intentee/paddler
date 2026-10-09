#![cfg(feature = "tests_that_use_llms")]

use paddler_tests::decision_cluster_params::DecisionClusterParams;

use crate::every_decision_question_answered_in_order::EveryDecisionQuestionAnsweredInOrder;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_5_answers_every_decision_question_in_order() {
    EveryDecisionQuestionAnsweredInOrder {
        decision_cluster_params: DecisionClusterParams::default(),
    }
    .assert_for_the_sample_decision()
    .await;
}
