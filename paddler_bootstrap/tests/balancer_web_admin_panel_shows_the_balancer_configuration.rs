use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::get;
use tokio_util::sync::CancellationToken;

use paddler_balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_test_cluster_harness::dashboard_attributes::dashboard_attributes;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

fn entered_as(input_addr: &str) -> ResolvedSocketAddr {
    ResolvedSocketAddr {
        input_addr: input_addr.to_owned(),
        socket_addr: EPHEMERAL_LOOPBACK_ADDR,
    }
}

#[tokio::test]
async fn balancer_web_admin_panel_shows_the_balancer_configuration() {
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.bootstrap_config.buffered_request_timeout = Duration::from_millis(1500);
    params.bootstrap_config.inference_service_configuration.addr = entered_as("localhost:0");
    params
        .bootstrap_config
        .management_service_configuration
        .addr = entered_as("[::ffff:127.0.0.1]:0");
    params.bootstrap_config.max_buffered_requests = 7;
    params.bootstrap_config.openai_service_configuration = Some(OpenAIServiceConfiguration {
        addr: entered_as("127.0.0.1:0"),
    });
    params.bootstrap_config.statsd_prefix = "dashboard_".to_owned();
    params.bootstrap_config.statsd_service_configuration = Some(StatsdServiceConfiguration {
        statsd_addr: entered_as("localhost:8125"),
        statsd_reporting_interval: Duration::from_hours(1),
    });
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
                ("data-buffered-request-timeout-millis", "1500"),
                ("data-compat-openai-addr", "127.0.0.1:0"),
                ("data-inference-addr", "localhost:0"),
                ("data-management-addr", "[::ffff:127.0.0.1]:0"),
                ("data-max-buffered-requests", "7"),
                ("data-statsd-addr", "localhost:8125"),
                ("data-statsd-prefix", "dashboard_"),
                ("data-statsd-reporting-interval-millis", "3600000"),
                ("id", "paddler-dashboard"),
            ]
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
        )
    );
}
