#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;

use crate::typesafe_tone_and_payment_decision::typesafe_tone_and_payment_decision;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_answers_questions_in_their_order() {
    let response = typesafe_tone_and_payment_decision().await;

    assert_eq!(response.status, StatusCode::OK);

    let answers = response.body["answers"]
        .as_object()
        .expect("the answers must be an object");

    assert_eq!(answers.keys().collect::<Vec<_>>(), vec!["tone", "paid"]);
    assert_eq!(answers["tone"]["type"], "choice");
    assert_eq!(
        answers["tone"]["probabilities"]
            .as_object()
            .expect("the choice probabilities must be an object")
            .keys()
            .collect::<Vec<_>>(),
        vec!["calm", "angry"]
    );
    assert_eq!(answers["paid"]["type"], "noul");
}
