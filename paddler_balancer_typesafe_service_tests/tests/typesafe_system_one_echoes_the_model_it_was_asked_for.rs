#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;

use paddler_balancer_typesafe_service_tests::typesafe_tone_and_payment_decision::typesafe_tone_and_payment_decision;
use paddler_cluster_harness::typesafe_request_model::typesafe_request_model;

use crate::constants::TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_echoes_the_model_it_was_asked_for() {
    let response = typesafe_tone_and_payment_decision(TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID)
        .await
        .expect("the tone and payment decision must be answered");

    assert_eq!(
        response.body.pointer("/model").and_then(Value::as_str),
        Some(typesafe_request_model())
    );
}
