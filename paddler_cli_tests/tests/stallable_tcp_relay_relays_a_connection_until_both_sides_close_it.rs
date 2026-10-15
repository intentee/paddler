use std::io;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use tokio::io::AsyncReadExt as _;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::spawn;

use paddler_cli_tests::stallable_tcp_relay::StallableTcpRelay;

#[tokio::test(flavor = "multi_thread")]
async fn stallable_tcp_relay_relays_a_connection_until_both_sides_close_it() {
    let upstream_listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .await
        .expect("the upstream must bind a loopback port");
    let upstream_addr = upstream_listener
        .local_addr()
        .expect("the upstream listener must know its address");
    let upstream = spawn(async move {
        let (mut relayed_connection, _relay_addr) = upstream_listener
            .accept()
            .await
            .expect("the upstream must accept the relayed connection");
        let mut request = Vec::new();

        relayed_connection
            .read_to_end(&mut request)
            .await
            .expect("the upstream must read the relayed request");
        relayed_connection
            .write_all(&request)
            .await
            .expect("the upstream must echo the request");
        relayed_connection
            .shutdown()
            .await
            .expect("the upstream must end its echo");
    });
    let relay_listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .await
        .expect("the relay must bind a loopback port");
    let relay_addr = relay_listener
        .local_addr()
        .expect("the relay listener must know its address");
    let relay = StallableTcpRelay::start(relay_listener, upstream_addr);
    let mut downstream = TcpStream::connect(relay_addr)
        .await
        .expect("the relay must accept a connection");
    let mut bytes_relayed_back = Vec::new();

    downstream
        .write_all(b"ping")
        .await
        .expect("the request must reach the relay");
    downstream.shutdown().await.expect("the request must end");
    downstream
        .read_to_end(&mut bytes_relayed_back)
        .await
        .expect("the relay must relay the echo back");

    assert_eq!(bytes_relayed_back, b"ping");
    upstream.await.expect("the upstream task must not panic");
    assert_eq!(
        relay
            .stop()
            .await
            .expect("the relay task must not panic")
            .err()
            .as_ref()
            .map(io::Error::kind),
        None
    );
}
