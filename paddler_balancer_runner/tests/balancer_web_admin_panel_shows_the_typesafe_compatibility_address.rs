use tokio_util::sync::CancellationToken;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer_runner::balancer_serving_mode::BalancerServingMode;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;
use crate::rendered_dashboard_attributes::rendered_dashboard_attributes;

#[tokio::test]
async fn balancer_web_admin_panel_shows_the_typesafe_compatibility_address() {
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.runner_config.serving_mode = BalancerServingMode::Decision {
        typesafe_service_configuration: Some(CompatibilityServiceConfiguration {
            addr: ResolvedSocketAddr {
                input_addr: "localhost:0".to_owned(),
                socket_addr: EPHEMERAL_LOOPBACK_ADDR,
            },
        }),
    };

    let dashboard_attributes = rendered_dashboard_attributes(params).await;

    assert_eq!(
        [
            dashboard_attributes.get("data-compat-typesafe-addr"),
            dashboard_attributes.get("data-inference-mode"),
        ],
        [
            Some(&"localhost:0".to_owned()),
            Some(&"Decision".to_owned())
        ]
    );
}
