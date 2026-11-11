use std::io;
use std::io::ErrorKind;

use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use paddler_local_http_fixture::resetting_client::ResettingClient;
use paddler_tcp_buffer_limits::tcp_buffer_limits_file::TcpBufferLimitsFile;

const WITHHELD_BYTE: u8 = 0;

#[tokio::test(flavor = "multi_thread")]
async fn a_client_resetting_during_a_stalled_body_ends_serving_with_a_connection_reset() {
    let client = ResettingClient::with_the_smallest_receive_buffer()
        .expect("the client must open its socket with the smallest receive buffer");
    let sent_body = client
        .body_outlasting_both_socket_buffers(&TcpBufferLimitsFile::of_kernel_send_buffers())
        .expect("the kernel must report its TCP send buffer limits");
    let fixture = LocalHttpFixture::start(FixtureResponse::StalledBody {
        sent_byte_count: sent_body.len(),
        declared_body: [sent_body, vec![WITHHELD_BYTE]].concat(),
    })
    .await
    .expect("the local HTTP fixture must start");

    client
        .request_then_reset(&fixture)
        .await
        .expect("the client must reset its connection after the fixture reads its request");

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
