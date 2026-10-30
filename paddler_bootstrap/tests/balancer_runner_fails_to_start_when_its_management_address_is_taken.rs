use std::io::ErrorKind;

use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::bootstrap_error::BootstrapError;

use crate::balancer_runner_start_on_a_taken_address::BalancerRunnerStartOnATakenAddress;

#[tokio::test]
async fn balancer_runner_fails_to_start_when_its_management_address_is_taken() {
    let start_attempt = BalancerRunnerStartOnATakenAddress::attempt(|params, taken_addr| {
        params
            .bootstrap_config
            .management_service_configuration
            .addr = ResolvedSocketAddr::from(taken_addr);
    })
    .await;

    assert!(matches!(
        start_attempt.start_error,
        BootstrapError::ManagementBindFailed { addr, .. } if addr == start_attempt.taken_addr
    ));
    assert_eq!(start_attempt.bind_error_kind(), Some(ErrorKind::AddrInUse));
}
