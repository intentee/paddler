use std::collections::BTreeMap;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer_runner::balancer_serving_mode::BalancerServingMode;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;
use crate::rendered_dashboard_attributes::rendered_dashboard_attributes;

#[tokio::test]
async fn balancer_web_admin_panel_omits_the_services_it_does_not_run() {
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.runner_config.buffered_request_timeout = Duration::from_secs(2);
    params.runner_config.max_buffered_requests = 3;
    params.runner_config.serving_mode = BalancerServingMode::Embeddings;
    params.runner_config.inference_service_configuration.addr = ResolvedSocketAddr {
        input_addr: "localhost:0".to_owned(),
        socket_addr: EPHEMERAL_LOOPBACK_ADDR,
    };
    params.runner_config.management_service_configuration.addr = ResolvedSocketAddr {
        input_addr: "localhost:0".to_owned(),
        socket_addr: EPHEMERAL_LOOPBACK_ADDR,
    };

    assert_eq!(
        rendered_dashboard_attributes(params).await,
        BTreeMap::from(
            [
                ("data-buffered-request-timeout-millis", "2000"),
                ("data-inference-addr", "localhost:0"),
                ("data-inference-mode", "Embeddings"),
                ("data-management-addr", "localhost:0"),
                ("data-max-buffered-requests", "3"),
                ("id", "paddler-dashboard"),
            ]
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
        )
    );
}
