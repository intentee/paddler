use std::io::Error as IoError;
use std::io::ErrorKind;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_bootstrap_tests::balancer_runner_start_on_a_taken_address::BalancerRunnerStartOnATakenAddress;

#[tokio::test]
async fn balancer_runner_fails_to_start_when_its_compat_typesafe_address_is_taken() {
    let start_attempt = BalancerRunnerStartOnATakenAddress::attempt(
        |BalancerRunnerParams {
             bootstrap_config, ..
         },
         taken_addr| {
            bootstrap_config.typesafe_service_configuration =
                Some(CompatibilityServiceConfiguration {
                    addr: ResolvedSocketAddr::from(taken_addr),
                });
        },
    )
    .await
    .expect("starting a runner on a taken address must fail");

    assert_eq!(
        start_attempt.start_error.to_string(),
        BootstrapError::CompatTypeSafeBindFailed {
            addr: start_attempt.taken_addr,
            source: IoError::from(ErrorKind::AddrInUse),
        }
        .to_string()
    );
    assert_eq!(start_attempt.bind_error_kind(), Some(ErrorKind::AddrInUse));
}
