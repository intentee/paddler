use std::io;
use std::io::ErrorKind;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use tokio::io::AsyncReadExt as _;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

use paddler_cli_tests::stallable_tcp_relay::StallableTcpRelay;
use paddler_cli_tests::unreachable_management_addr::UNREACHABLE_MANAGEMENT_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn stallable_tcp_relay_reports_an_upstream_it_cannot_reach() {
    let relay_listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .await
        .expect("the relay must bind a loopback port");
    let relay_addr = relay_listener
        .local_addr()
        .expect("the relay listener must know its address");
    let relay = StallableTcpRelay::start(relay_listener, UNREACHABLE_MANAGEMENT_ADDR);
    let mut downstream = TcpStream::connect(relay_addr)
        .await
        .expect("the relay must accept a connection");
    let mut bytes_relayed_back = Vec::new();

    downstream
        .read_to_end(&mut bytes_relayed_back)
        .await
        .expect("the relay must close a connection it cannot relay");

    assert_eq!(
        relay
            .stop()
            .await
            .expect("the relay task must not panic")
            .err()
            .as_ref()
            .map(io::Error::kind),
        Some(ErrorKind::ConnectionRefused)
    );
}
