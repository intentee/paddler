#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;

use paddler_tests::typesafe_tone_and_payment_decision::typesafe_tone_and_payment_decision;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_that_a_decision_produces_no_output_tokens() {
    let response = typesafe_tone_and_payment_decision()
        .await
        .expect("the tone and payment decision must be answered");

    assert_eq!(
        response
            .body
            .pointer("/usage/output_tokens")
            .and_then(Value::as_i64),
        Some(0)
    );
}
