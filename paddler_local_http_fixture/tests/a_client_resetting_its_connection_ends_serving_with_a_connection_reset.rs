use std::io;
use std::io::ErrorKind;

use tokio::net::TcpStream;

use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

#[tokio::test]
async fn a_client_resetting_its_connection_ends_serving_with_a_connection_reset() {
    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(Vec::new()))
        .await
        .expect("the local HTTP fixture must start");
    let client = TcpStream::connect(fixture.addr)
        .await
        .expect("the client must connect to the fixture");

    client
        .set_zero_linger()
        .expect("the client must reset its connection when it closes");
    drop(client);

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
