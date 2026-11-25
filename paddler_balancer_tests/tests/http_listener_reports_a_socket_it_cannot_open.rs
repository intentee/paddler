use std::fs::File;
use std::io;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::os::fd::AsRawFd as _;

use nix::errno::Errno;
use nix::sys::resource::Resource;
use nix::sys::resource::getrlimit;
use nix::sys::resource::setrlimit;

use paddler_balancer::http_listener::HttpListener;

#[test]
fn http_listener_reports_a_socket_it_cannot_open() {
    let lowest_free_descriptor = File::open("/dev/null")
        .expect("a probe descriptor must open")
        .as_raw_fd();
    let (soft_descriptor_limit, hard_descriptor_limit) =
        getrlimit(Resource::RLIMIT_NOFILE).expect("the descriptor limit must be readable");

    setrlimit(
        Resource::RLIMIT_NOFILE,
        u64::try_from(lowest_free_descriptor).expect("descriptors are never negative"),
        hard_descriptor_limit,
    )
    .expect("the descriptor limit must be lowered");

    let bind_errno = HttpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .err()
        .as_ref()
        .and_then(io::Error::raw_os_error)
        .map(Errno::from_raw);

    setrlimit(
        Resource::RLIMIT_NOFILE,
        soft_descriptor_limit,
        hard_descriptor_limit,
    )
    .expect("the descriptor limit must be restored");

    assert_eq!(bind_errno, Some(Errno::EMFILE));
}
