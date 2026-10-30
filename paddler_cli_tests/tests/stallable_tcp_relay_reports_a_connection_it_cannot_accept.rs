use std::fs::File;
use std::io;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::os::fd::AsRawFd as _;

use nix::errno::Errno;
use nix::sys::resource::Resource;
use nix::sys::resource::getrlimit;
use nix::sys::resource::setrlimit;
use nix::sys::socket::AddressFamily;
use nix::sys::socket::SockFlag;
use nix::sys::socket::SockType;
use nix::sys::socket::SockaddrIn;
use nix::sys::socket::connect;
use nix::sys::socket::socket;
use tokio::net::TcpListener;

use paddler_cli_tests::stallable_tcp_relay::StallableTcpRelay;
use paddler_cli_tests::unreachable_management_addr::UNREACHABLE_MANAGEMENT_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn stallable_tcp_relay_reports_a_connection_it_cannot_accept() {
    let relay_listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .await
        .expect("the relay must bind a loopback port");
    let relay_addr_v4 = SocketAddrV4::new(
        Ipv4Addr::LOCALHOST,
        relay_listener
            .local_addr()
            .expect("the relay listener must know its address")
            .port(),
    );
    let relay = StallableTcpRelay::start(relay_listener, UNREACHABLE_MANAGEMENT_ADDR);
    let downstream_socket = socket(
        AddressFamily::Inet,
        SockType::Stream,
        SockFlag::empty(),
        None,
    )
    .expect("the downstream socket must open");
    let lowest_free_descriptor = File::open("/dev/null")
        .expect("a probe descriptor must open")
        .as_raw_fd();
    let (_soft_descriptor_limit, hard_descriptor_limit) =
        getrlimit(Resource::RLIMIT_NOFILE).expect("the descriptor limit must be readable");

    setrlimit(
        Resource::RLIMIT_NOFILE,
        u64::try_from(lowest_free_descriptor).expect("descriptors are never negative"),
        hard_descriptor_limit,
    )
    .expect("the descriptor limit must be lowered");

    connect(
        downstream_socket.as_raw_fd(),
        &SockaddrIn::from(relay_addr_v4),
    )
    .expect("the kernel must complete the handshake without a new descriptor");

    assert_eq!(
        relay
            .wait_until_relaying_ends()
            .await
            .expect("the relay task must not panic")
            .err()
            .as_ref()
            .and_then(io::Error::raw_os_error)
            .map(Errno::from_raw),
        Some(Errno::EMFILE)
    );
}
