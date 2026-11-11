use std::collections::BTreeMap;

use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use reqwest::get;

use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_test_cluster_harness::constants::EPHEMERAL_LOOPBACK_ADDR;
use paddler_test_cluster_harness::dashboard_attributes::dashboard_attributes;

use crate::bootstrap_tests_error::BootstrapTestsError;

async fn fetch_the_dashboard_then_stop(
    mut runner: BalancerRunner,
) -> Result<String, BootstrapTestsError> {
    ready(
        runner
            .addresses
            .web_admin_panel
            .ok_or(BootstrapTestsError::WebAdminPanelAddrUnreported),
    )
    .and_then(|web_admin_panel_addr| {
        get(format!("http://{web_admin_panel_addr}/"))
            .map_err(BootstrapTestsError::DashboardUnreachable)
    })
    .and_then(|response| {
        ready(
            response
                .error_for_status()
                .map_err(BootstrapTestsError::DashboardNotRendered),
        )
    })
    .and_then(|response| {
        response
            .text()
            .map_err(BootstrapTestsError::DashboardBodyUnreadable)
    })
    .and_then(|dashboard| {
        runner.cancel();
        runner
            .wait_for_completion()
            .map_ok(|()| dashboard)
            .map_err(BootstrapTestsError::RunnerShutdownFailed)
    })
    .await
}

/// # Errors
///
/// Returns [`BootstrapTestsError::RunnerStartFailed`] when the balancer runner does not start, [`BootstrapTestsError::WebAdminPanelAddrUnreported`] when it does not report the web admin panel address, [`BootstrapTestsError::DashboardUnreachable`] when the web admin panel does not accept connections, [`BootstrapTestsError::DashboardNotRendered`] when it does not render the dashboard, [`BootstrapTestsError::DashboardBodyUnreadable`] when the dashboard body cannot be read, [`BootstrapTestsError::RunnerShutdownFailed`] when the runner does not shut down cleanly, and [`BootstrapTestsError::DashboardAttributesUnreadable`] when the dashboard does not carry its configuration.
pub async fn rendered_dashboard_attributes(
    mut params: BalancerRunnerParams,
) -> Result<BTreeMap<String, String>, BootstrapTestsError> {
    params
        .bootstrap_config
        .web_admin_panel_service_configuration = Some(WebAdminPanelServiceConfiguration {
        addr: EPHEMERAL_LOOPBACK_ADDR,
    });

    BalancerRunner::start(params)
        .map_err(BootstrapTestsError::RunnerStartFailed)
        .and_then(fetch_the_dashboard_then_stop)
        .await
        .and_then(|dashboard| {
            dashboard_attributes(&dashboard)
                .map_err(BootstrapTestsError::DashboardAttributesUnreadable)
        })
}
