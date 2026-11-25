use std::io::ErrorKind;

use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_bootstrap_tests::balancer_runner_start_on_a_taken_address::BalancerRunnerStartOnATakenAddress;

#[tokio::test]
async fn balancer_runner_fails_to_start_when_its_web_admin_panel_address_is_taken() {
    let start_attempt = BalancerRunnerStartOnATakenAddress::attempt(
        |BalancerRunnerParams {
             bootstrap_config, ..
         },
         taken_addr| {
            bootstrap_config.web_admin_panel_service_configuration =
                Some(WebAdminPanelServiceConfiguration { addr: taken_addr });
        },
    )
    .await
    .expect("starting a runner on a taken address must fail");

    assert!(matches!(
        start_attempt.start_error,
        BootstrapError::WebAdminPanelBindFailed { addr, .. } if addr == start_attempt.taken_addr
    ));
    assert_eq!(start_attempt.bind_error_kind(), Some(ErrorKind::AddrInUse));
}
