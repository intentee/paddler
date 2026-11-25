#![cfg(feature = "tests_that_use_llms")]

use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_answer::DecisionAnswer;
use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::Value;
use serde_json::json;

use paddler_agent_decision_tests::decided_by_a_cluster_serving::decided_by_a_cluster_serving;
use paddler_file_fixture::constants::LLAMAS_JPEG;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_model_card_fixture::minicpm_ocr_jev::minicpm_ocr_jev;
use paddler_model_card_fixture::minicpm_ocr_jev_mmproj::minicpm_ocr_jev_mmproj;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_model_reading_images_scores_every_class_of_an_image_decision() {
    let llamas = LLAMAS_JPEG.data_uri().expect("the test image must load");
    let collected = decided_by_a_cluster_serving(
        minicpm_ocr_jev().into_desired_state_with_multimodal_projection(minicpm_ocr_jev_mmproj()),
        &DecisionRequest {
            images: vec![llamas.clone(), llamas],
            questions: IndexMap::from([(
                "consistency".to_owned(),
                DecisionQuestion::Choice {
                    criteria: IndexMap::from([
                        ("0".to_owned(), Value::Null),
                        ("1".to_owned(), Value::Null),
                    ]),
                    instructions: Value::Null,
                },
            )]),
            state: json!("Two llamas stand on a grassy hill."),
        },
    )
    .await
    .expect("the decision must be answered");

    assert!(
        collected
            .answers
            .iter()
            .any(|AnsweredDecisionQuestion { answer, id }| {
                id == "consistency"
                    && matches!(
                        answer,
                        DecisionAnswer::Choice { probabilities, .. }
                            if probabilities.contains_key("0") && probabilities.contains_key("1")
                    )
            }),
        "answers were {:?}",
        collected.answers
    );
}
