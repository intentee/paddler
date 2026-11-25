#![cfg(feature = "tests_that_use_llms")]

use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_answer::DecisionAnswer;
use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use serde_json::json;

use paddler_agent_decision_tests::chose_option::chose_option;
use paddler_agent_decision_tests::decided_by_a_cluster_serving::decided_by_a_cluster_serving;
use paddler_agent_decision_tests::snowy_weather_decision::snowy_weather_decision;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_model_card_fixture::clef_flash::clef_flash;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_model_answering_every_question_in_one_prompt_answers_each_of_them() {
    let mut request = snowy_weather_decision();

    request.questions.insert(
        "freezing".to_owned(),
        DecisionQuestion::Noul {
            criteria: IndexMap::new(),
            instructions: json!("Is it below freezing outside?"),
        },
    );

    let collected = decided_by_a_cluster_serving(clef_flash().into_desired_state(), &request)
        .await
        .expect("the decision must be answered");

    assert!(
        chose_option(&collected.answers, "weather", "snowing")
            && collected
                .answers
                .iter()
                .any(|AnsweredDecisionQuestion { answer, id }| {
                    id == "freezing"
                        && matches!(answer, DecisionAnswer::Noul { noul } if *noul > 0.5)
                }),
        "answers were {:?}",
        collected.answers
    );
}
