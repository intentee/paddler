use std::fs::File;
use std::io;
use std::net::Ipv4Addr;
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

use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;

#[tokio::test(flavor = "multi_thread")]
async fn a_connection_the_fixture_cannot_accept_ends_serving() {
    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(Vec::new()))
        .await
        .expect("the local HTTP fixture must start");
    let client_socket = socket(
        AddressFamily::Inet,
        SockType::Stream,
        SockFlag::empty(),
        None,
    )
    .expect("the client socket must open");
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
        client_socket.as_raw_fd(),
        &SockaddrIn::from(SocketAddrV4::new(Ipv4Addr::LOCALHOST, fixture.addr.port())),
    )
    .expect("the kernel must complete the handshake without a new descriptor");

    assert_eq!(
        fixture
            .wait_until_serving_fails()
            .await
            .expect("the fixture task must not panic")
            .err()
            .as_ref()
            .and_then(io::Error::raw_os_error)
            .map(Errno::from_raw),
        Some(Errno::EMFILE)
    );
}
