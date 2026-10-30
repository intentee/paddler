use std::io;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use libc::EPERM;
use libc::SYS_setsockopt;

use paddler_balancer::http_listener::HttpListener;

use crate::refused_syscall::RefusedSyscall;
use crate::refused_syscalls::RefusedSyscalls;

#[test]
fn http_listener_reports_a_reuse_address_option_it_cannot_set() {
    RefusedSyscalls {
        errno: EPERM,
        refused_syscalls: vec![RefusedSyscall {
            syscall: SYS_setsockopt,
            rules: Vec::new(),
        }],
    }
    .apply_to_every_thread();

    assert_eq!(
        HttpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .err()
            .as_ref()
            .and_then(io::Error::raw_os_error),
        Some(EPERM)
    );
}
