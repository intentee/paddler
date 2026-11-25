use std::fs::write;

use tempfile::TempDir;

use paddler_local_http_fixture::resetting_client::ResettingClient;
use paddler_local_http_fixture::resetting_client_error::ResettingClientError;
use paddler_tcp_buffer_limits::tcp_buffer_limits_file::TcpBufferLimitsFile;

#[test]
fn a_send_buffer_limit_no_body_can_outlast_is_reported() {
    let client = ResettingClient::with_the_smallest_receive_buffer()
        .expect("the client must open its socket with the smallest receive buffer");
    let limits_directory = TempDir::new().expect("a temporary directory must be creatable");
    let send_buffer_limits_path = limits_directory.path().join("tcp_wmem");

    write(
        &send_buffer_limits_path,
        format!("4096\t16384\t{}\n", usize::MAX),
    )
    .expect("the send buffer limits must be writable");

    assert!(matches!(
        client.body_outlasting_both_socket_buffers(&TcpBufferLimitsFile {
            path: send_buffer_limits_path,
        }),
        Err(ResettingClientError::OutlastingBodyLengthOverflows {
            largest_send_buffer,
            ..
        }) if largest_send_buffer == usize::MAX
    ));
}
