use std::io;
use std::io::ErrorKind;

use httparse::Error as HttpParseError;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpStream;

use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

#[tokio::test]
async fn a_malformed_request_ends_serving_with_invalid_data() {
    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(Vec::new()))
        .await
        .expect("the local HTTP fixture must start");
    let mut client = TcpStream::connect(fixture.addr)
        .await
        .expect("the client must connect to the fixture");

    client
        .write_all(b"not an http request\r\n\r\n")
        .await
        .expect("the client must send its malformed request");

    let serving_error = fixture
        .wait_until_serving_fails()
        .await
        .expect("the fixture task must not panic")
        .err();

    assert_eq!(
        serving_error.as_ref().map(io::Error::kind),
        Some(ErrorKind::InvalidData)
    );
    assert_eq!(
        serving_error
            .as_ref()
            .and_then(io::Error::get_ref)
            .and_then(|parse_error| parse_error.downcast_ref::<HttpParseError>()),
        Some(&HttpParseError::Version)
    );
}
