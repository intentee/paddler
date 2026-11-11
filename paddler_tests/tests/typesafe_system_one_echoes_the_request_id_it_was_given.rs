#![cfg(feature = "tests_that_use_llms")]

use paddler_balancer::constants::TYPESAFE_HEADER_REQUEST_ID;
use paddler_tests::constants::TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID;
use paddler_tests::typesafe_tone_and_payment_decision::typesafe_tone_and_payment_decision;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_echoes_the_request_id_it_was_given() {
    let response = typesafe_tone_and_payment_decision()
        .await
        .expect("the tone and payment decision must be answered");

    assert_eq!(
        response.headers.get(TYPESAFE_HEADER_REQUEST_ID),
        Some(
            &TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID
                .parse()
                .expect("the header value is valid")
        )
    );
}
