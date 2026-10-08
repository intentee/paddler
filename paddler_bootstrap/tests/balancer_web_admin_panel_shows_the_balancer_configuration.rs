use std::collections::BTreeMap;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;
use crate::rendered_dashboard_attributes::rendered_dashboard_attributes;

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
    params.bootstrap_config.openai_service_configuration =
        Some(CompatibilityServiceConfiguration {
            addr: entered_as("127.0.0.1:0"),
        });
    params.bootstrap_config.statsd_prefix = "dashboard_".to_owned();
    params.bootstrap_config.statsd_service_configuration = Some(StatsdServiceConfiguration {
        statsd_addr: entered_as("localhost:8125"),
        statsd_reporting_interval: Duration::from_hours(1),
    });
    params.bootstrap_config.typesafe_service_configuration =
        Some(CompatibilityServiceConfiguration {
            addr: entered_as("[::1]:0"),
        });

    assert_eq!(
        rendered_dashboard_attributes(params).await,
        BTreeMap::from(
            [
                ("data-buffered-request-timeout-millis", "1500"),
                ("data-compat-openai-addr", "127.0.0.1:0"),
                ("data-compat-typesafe-addr", "[::1]:0"),
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
