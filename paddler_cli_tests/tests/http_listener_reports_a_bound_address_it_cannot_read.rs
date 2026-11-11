use std::io;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use libc::EPERM;
use libc::SYS_getsockname;

use paddler_balancer::http_listener::HttpListener;
use paddler_cli_tests::refused_syscall::RefusedSyscall;
use paddler_cli_tests::refused_syscalls::RefusedSyscalls;

#[test]
fn http_listener_reports_a_bound_address_it_cannot_read() {
    RefusedSyscalls {
        errno: EPERM,
        refused_syscalls: vec![RefusedSyscall {
            syscall: SYS_getsockname,
            rules: Vec::new(),
        }],
    }
    .apply_to_every_thread()
    .expect("the refusing filter must apply to every thread of the test process");

    assert_eq!(
        HttpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .err()
            .as_ref()
            .and_then(io::Error::raw_os_error),
        Some(EPERM)
    );
}
