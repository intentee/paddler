use std::io;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use actix_web::App;
use libc::EPERM;
use libc::FIONBIO;
use libc::SYS_ioctl;
use seccompiler::SeccompCmpArgLen;
use seccompiler::SeccompCmpOp;
use seccompiler::SeccompCondition;
use seccompiler::SeccompRule;
use tokio_util::sync::CancellationToken;

use paddler_balancer::http_listener::HttpListener;
use paddler_balancer::run_http_service::run_http_service;
use paddler_balancer::run_http_service_parameters::RunHttpServiceParameters;

use crate::refused_syscall::RefusedSyscall;
use crate::refused_syscalls::RefusedSyscalls;

const SERVICE_NAME: &str = "balancer::service_that_cannot_listen";

#[actix_web::test]
async fn an_http_service_reports_a_listener_it_cannot_make_nonblocking() {
    let http_listener = HttpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .expect("the listener must bind before nonblocking mode is refused");
    let local_addr = http_listener.local_addr;

    RefusedSyscalls {
        errno: EPERM,
        refused_syscalls: vec![RefusedSyscall {
            syscall: SYS_ioctl,
            rules: vec![
                SeccompRule::new(vec![
                    SeccompCondition::new(1, SeccompCmpArgLen::Dword, SeccompCmpOp::Eq, FIONBIO)
                        .expect("the nonblocking request condition must be valid"),
                ])
                .expect("the nonblocking request rule must be valid"),
            ],
        }],
    }
    .apply_to_every_thread();

    let service_error = run_http_service(
        CancellationToken::new(),
        RunHttpServiceParameters {
            app_factory: App::new,
            http_listener,
            service_name: SERVICE_NAME,
            worker_count: 1,
        },
    )
    .await
    .expect_err("a listener that cannot become nonblocking must not be served");

    assert_eq!(
        service_error.to_string(),
        format!("Unable to listen for {SERVICE_NAME} on {local_addr}")
    );
    assert_eq!(
        service_error
            .root_cause()
            .downcast_ref::<io::Error>()
            .and_then(io::Error::raw_os_error),
        Some(EPERM)
    );
}
