#![cfg(feature = "tests_that_use_llms")]

use paddler_test_cluster_harness::typesafe_request_model::TYPESAFE_REQUEST_MODEL;

use crate::typesafe_tone_and_payment_decision::typesafe_tone_and_payment_decision;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_echoes_the_model_it_was_asked_for() {
    let response = typesafe_tone_and_payment_decision().await;

    assert_eq!(response.body["model"], TYPESAFE_REQUEST_MODEL);
}
