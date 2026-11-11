#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;

use paddler_test_cluster_harness::constants::TYPESAFE_REQUEST_MODEL;
use paddler_tests::typesafe_tone_and_payment_decision::typesafe_tone_and_payment_decision;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_echoes_the_model_it_was_asked_for() {
    let response = typesafe_tone_and_payment_decision()
        .await
        .expect("the tone and payment decision must be answered");

    assert_eq!(
        response.body.pointer("/model").and_then(Value::as_str),
        Some(TYPESAFE_REQUEST_MODEL)
    );
}
