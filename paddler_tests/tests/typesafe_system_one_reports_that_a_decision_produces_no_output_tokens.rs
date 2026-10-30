#![cfg(feature = "tests_that_use_llms")]

use crate::typesafe_tone_and_payment_decision::typesafe_tone_and_payment_decision;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_that_a_decision_produces_no_output_tokens() {
    let response = typesafe_tone_and_payment_decision().await;

    assert_eq!(response.body["usage"]["output_tokens"], 0);
}
