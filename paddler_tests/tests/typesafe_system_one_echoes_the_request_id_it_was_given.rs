#![cfg(feature = "tests_that_use_llms")]

use paddler_balancer::compatibility::typesafe_service::typesafe_header::TypeSafeHeader;

use crate::typesafe_tone_and_payment_decision::TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID;
use crate::typesafe_tone_and_payment_decision::typesafe_tone_and_payment_decision;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_echoes_the_request_id_it_was_given() {
    let response = typesafe_tone_and_payment_decision().await;

    assert_eq!(
        response.headers.get(TypeSafeHeader::REQUEST_ID),
        Some(
            &TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID
                .parse()
                .expect("the header value is valid")
        )
    );
}
