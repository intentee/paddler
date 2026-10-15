use std::io;
use std::io::ErrorKind;

use tokio::net::TcpStream;

use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

#[tokio::test]
async fn a_client_leaving_before_its_request_ends_serving_with_an_unexpected_eof() {
    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(Vec::new()))
        .await
        .expect("the local HTTP fixture must start");

    drop(
        TcpStream::connect(fixture.addr)
            .await
            .expect("the client must connect to the fixture"),
    );

    assert_eq!(
        fixture
            .wait_until_serving_fails()
            .await
            .expect("the fixture task must not panic")
            .err()
            .as_ref()
            .map(io::Error::kind),
        Some(ErrorKind::UnexpectedEof)
    );
}
