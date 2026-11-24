#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use serde_json::Value;

use paddler_tests::typesafe_tone_and_payment_decision::typesafe_tone_and_payment_decision;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_answers_questions_in_their_order() {
    let response = typesafe_tone_and_payment_decision()
        .await
        .expect("the tone and payment decision must be answered");

    assert_eq!(response.status, StatusCode::OK);

    let answers = response
        .body
        .pointer("/answers")
        .and_then(Value::as_object)
        .expect("the answers must be an object");

    assert_eq!(answers.keys().collect::<Vec<_>>(), vec!["tone", "paid"]);
    assert_eq!(
        response
            .body
            .pointer("/answers/tone/type")
            .and_then(Value::as_str),
        Some("choice")
    );
    assert_eq!(
        response
            .body
            .pointer("/answers/tone/probabilities")
            .and_then(Value::as_object)
            .expect("the choice probabilities must be an object")
            .keys()
            .collect::<Vec<_>>(),
        vec!["calm", "angry"]
    );
    assert_eq!(
        response
            .body
            .pointer("/answers/paid/type")
            .and_then(Value::as_str),
        Some("noul")
    );
}
