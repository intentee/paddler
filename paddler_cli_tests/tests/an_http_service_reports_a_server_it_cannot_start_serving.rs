use std::net::Ipv4Addr;
use std::net::SocketAddr;

use actix_web::App;
use libc::EPERM;
use libc::SYS_epoll_create1;
use tokio_util::sync::CancellationToken;

use paddler_balancer::balancer_error::BalancerError;
use paddler_balancer::http_listener::HttpListener;
use paddler_balancer::http_service_runner::HttpServiceRunner;
use paddler_cli_tests::refused_syscall::RefusedSyscall;
use paddler_cli_tests::refused_syscalls::RefusedSyscalls;

const SERVICE_NAME: &str = "balancer::service_that_cannot_start_serving";

#[actix_web::test]
async fn an_http_service_reports_a_server_it_cannot_start_serving() {
    let http_listener = HttpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .expect("the listener must bind before the event poll is refused");
    let local_addr = http_listener.local_addr;

    RefusedSyscalls {
        errno: EPERM,
        refused_syscalls: vec![RefusedSyscall {
            syscall: SYS_epoll_create1,
            rules: vec![],
        }],
    }
    .apply_to_every_thread()
    .expect("the refusing filter must apply to every thread of the test process");

    let service_error = HttpServiceRunner {
        app_factory: App::new,
        http_listener,
        service_name: SERVICE_NAME,
        worker_count: 1,
    }
    .run(CancellationToken::new())
    .await
    .expect_err("a server that cannot create its event poll must not serve");

    assert_eq!(
        service_error.to_string(),
        format!("Unable to serve {SERVICE_NAME} on {local_addr}")
    );
    assert!(matches!(
        service_error,
        BalancerError::HttpServiceFailed { source, .. } if source.raw_os_error() == Some(EPERM)
    ));
}
