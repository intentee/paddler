#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings_types::decision_answer::DecisionAnswer;

use paddler_agent_decision_tests::decided_by_a_cluster_serving::decided_by_a_cluster_serving;
use paddler_agent_decision_tests::snowy_weather_and_freezing_decision::snowy_weather_and_freezing_decision;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_model_card_fixture::tiny_decision_2_0_kai_0_6b_for_testing::tiny_decision_2_0_kai_0_6b_for_testing;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_model_reading_each_question_from_its_own_joint_prompt_answers_each_of_them() {
    let collected = decided_by_a_cluster_serving(
        tiny_decision_2_0_kai_0_6b_for_testing().into_desired_state(),
        &snowy_weather_and_freezing_decision(),
    )
    .await
    .expect("the decision must be answered");

    assert!(
        matches!(
            collected.answers.as_slice(),
            [
                AnsweredDecisionQuestion {
                    answer: DecisionAnswer::Choice { probabilities, .. },
                    id: weather_id,
                },
                AnsweredDecisionQuestion {
                    answer: DecisionAnswer::Noul { .. },
                    id: freezing_id,
                },
            ] if weather_id == "weather"
                && probabilities.keys().eq(["sunny", "raining", "snowing"])
                && freezing_id == "freezing"
        ),
        "answers were {:?}",
        collected.answers
    );
}
