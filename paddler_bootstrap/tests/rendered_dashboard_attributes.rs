use std::collections::BTreeMap;

use reqwest::get;

use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_test_cluster_harness::dashboard_attributes::dashboard_attributes;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

pub async fn rendered_dashboard_attributes(
    mut params: BalancerRunnerParams,
) -> BTreeMap<String, String> {
    params
        .bootstrap_config
        .web_admin_panel_service_configuration = Some(WebAdminPanelServiceConfiguration {
        addr: EPHEMERAL_LOOPBACK_ADDR,
    });

    let mut runner = BalancerRunner::start(params)
        .await
        .expect("a runner with the web admin panel must start");
    let web_admin_panel_addr = runner
        .addresses
        .web_admin_panel
        .expect("the runner must report the web admin panel address");
    let dashboard = get(format!("http://{web_admin_panel_addr}/"))
        .await
        .expect("the web admin panel must accept connections")
        .error_for_status()
        .expect("the web admin panel must render the dashboard")
        .text()
        .await
        .expect("the dashboard must have a body");

    runner.cancel();
    runner
        .wait_for_completion()
        .await
        .expect("the runner must shut down cleanly");

    dashboard_attributes(&dashboard).expect("the dashboard must carry its configuration")
}
