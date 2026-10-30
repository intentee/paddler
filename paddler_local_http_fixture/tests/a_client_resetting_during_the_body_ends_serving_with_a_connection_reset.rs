use std::io;
use std::io::ErrorKind;

use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

use crate::resetting_client::ResettingClient;

#[tokio::test(flavor = "multi_thread")]
async fn a_client_resetting_during_the_body_ends_serving_with_a_connection_reset() {
    let client = ResettingClient::with_the_smallest_receive_buffer();
    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(
        client.body_outlasting_both_socket_buffers(),
    ))
    .await
    .expect("the local HTTP fixture must start");

    client.request_then_reset(&fixture).await;

    assert_eq!(
        fixture
            .wait_until_serving_fails()
            .await
            .expect("the fixture task must not panic")
            .err()
            .as_ref()
            .map(io::Error::kind),
        Some(ErrorKind::ConnectionReset)
    );
}
