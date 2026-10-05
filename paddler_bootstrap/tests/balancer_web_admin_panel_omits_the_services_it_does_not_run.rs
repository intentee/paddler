use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::get;
use tokio_util::sync::CancellationToken;

use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_test_cluster_harness::dashboard_attributes::dashboard_attributes;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_web_admin_panel_omits_the_services_it_does_not_run() {
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.bootstrap_config.buffered_request_timeout = Duration::from_secs(2);
    params.bootstrap_config.max_buffered_requests = 3;
    params.bootstrap_config.inference_service_configuration.addr = ResolvedSocketAddr {
        input_addr: "localhost:0".to_owned(),
        socket_addr: EPHEMERAL_LOOPBACK_ADDR,
    };
    params
        .bootstrap_config
        .management_service_configuration
        .addr = ResolvedSocketAddr {
        input_addr: "localhost:0".to_owned(),
        socket_addr: EPHEMERAL_LOOPBACK_ADDR,
    };
    params
        .bootstrap_config
        .web_admin_panel_service_configuration = Some(WebAdminPanelServiceConfiguration {
        addr: EPHEMERAL_LOOPBACK_ADDR,
    });

    let runner = BalancerRunner::start(params)
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

    assert_eq!(
        dashboard_attributes(&dashboard).expect("the dashboard must carry its configuration"),
        BTreeMap::from(
            [
                ("data-buffered-request-timeout-millis", "2000"),
                ("data-inference-addr", "localhost:0"),
                ("data-management-addr", "localhost:0"),
                ("data-max-buffered-requests", "3"),
                ("id", "paddler-dashboard"),
            ]
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
        )
    );
}
