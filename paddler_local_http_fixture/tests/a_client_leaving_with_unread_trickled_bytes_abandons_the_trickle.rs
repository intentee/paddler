use std::io::ErrorKind;
use std::time::Duration;

use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpStream;

use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

const BYTE_INTERVAL_AFTER_THE_IMMEDIATE_FIRST_BYTE: Duration = Duration::from_secs(1);

#[tokio::test]
async fn a_client_leaving_with_unread_trickled_bytes_abandons_the_trickle() {
    let fixture = LocalHttpFixture::start(FixtureResponse::EndlessTrickle {
        byte_interval: BYTE_INTERVAL_AFTER_THE_IMMEDIATE_FIRST_BYTE,
    })
    .await
    .expect("the local HTTP fixture must start");
    let mut client = TcpStream::connect(fixture.addr)
        .await
        .expect("the client must connect to the fixture");

    client
        .write_all(b"GET /endless.gguf HTTP/1.1\r\nHost: fixture\r\n\r\n")
        .await
        .expect("the client must send its request");
    client
        .peek(&mut [0; 1])
        .await
        .expect("the fixture must start trickling the response");
    drop(client);

    fixture
        .wait_for_abandoned_trickles(1)
        .await
        .expect("the fixture must record the client leaving the trickle");

    assert_eq!(
        fixture.abandoned_trickle_departures(),
        vec![ErrorKind::ConnectionReset]
    );

    fixture
        .stop()
        .await
        .expect("the fixture task must not panic")
        .expect("the fixture must serve every request");
}
