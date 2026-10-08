use std::net::TcpListener;

use tokio_util::sync::CancellationToken;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

async fn start_error_of(params: BalancerRunnerParams) -> Option<BootstrapError> {
    BalancerRunner::start(params).await.err()
}

#[tokio::test]
async fn balancer_runner_fails_to_start_when_its_inference_address_is_taken() {
    let occupying_listener =
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR).expect("an ephemeral loopback port must bind");
    let taken_addr = occupying_listener
        .local_addr()
        .expect("a bound listener must report its address");
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.bootstrap_config.inference_service_configuration.addr =
        ResolvedSocketAddr::from(taken_addr);

    assert!(matches!(
        start_error_of(params).await,
        Some(BootstrapError::InferenceBindFailed { addr, .. }) if addr == taken_addr
    ));
}

#[tokio::test]
async fn balancer_runner_fails_to_start_when_its_management_address_is_taken() {
    let occupying_listener =
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR).expect("an ephemeral loopback port must bind");
    let taken_addr = occupying_listener
        .local_addr()
        .expect("a bound listener must report its address");
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params
        .bootstrap_config
        .management_service_configuration
        .addr = ResolvedSocketAddr::from(taken_addr);

    assert!(matches!(
        start_error_of(params).await,
        Some(BootstrapError::ManagementBindFailed { addr, .. }) if addr == taken_addr
    ));
}

#[tokio::test]
async fn balancer_runner_fails_to_start_when_its_compat_openai_address_is_taken() {
    let occupying_listener =
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR).expect("an ephemeral loopback port must bind");
    let taken_addr = occupying_listener
        .local_addr()
        .expect("a bound listener must report its address");
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.bootstrap_config.openai_service_configuration =
        Some(CompatibilityServiceConfiguration {
            addr: ResolvedSocketAddr::from(taken_addr),
        });

    assert!(matches!(
        start_error_of(params).await,
        Some(BootstrapError::CompatOpenAIBindFailed { addr, .. }) if addr == taken_addr
    ));
}

#[tokio::test]
async fn balancer_runner_fails_to_start_when_its_compat_typesafe_address_is_taken() {
    let occupying_listener =
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR).expect("an ephemeral loopback port must bind");
    let taken_addr = occupying_listener
        .local_addr()
        .expect("a bound listener must report its address");
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.bootstrap_config.typesafe_service_configuration =
        Some(CompatibilityServiceConfiguration {
            addr: ResolvedSocketAddr::from(taken_addr),
        });

    assert!(matches!(
        start_error_of(params).await,
        Some(BootstrapError::CompatTypeSafeBindFailed { addr, .. }) if addr == taken_addr
    ));
}

#[cfg(feature = "web_admin_panel")]
#[tokio::test]
async fn balancer_runner_fails_to_start_when_its_web_admin_panel_address_is_taken() {
    let occupying_listener =
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR).expect("an ephemeral loopback port must bind");
    let taken_addr = occupying_listener
        .local_addr()
        .expect("a bound listener must report its address");
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params
        .bootstrap_config
        .web_admin_panel_service_configuration =
        Some(WebAdminPanelServiceConfiguration { addr: taken_addr });

    assert!(matches!(
        start_error_of(params).await,
        Some(BootstrapError::WebAdminPanelBindFailed { addr, .. }) if addr == taken_addr
    ));
}
