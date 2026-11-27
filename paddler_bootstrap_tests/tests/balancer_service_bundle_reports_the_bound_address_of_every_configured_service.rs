use std::time::Duration;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_balancer_state_database::state_database_type::StateDatabaseType;
use paddler_balancer_state_database::stored_balancer_desired_state::StoredBalancerDesiredState;
use paddler_bootstrap::balancer_bootstrap_config::BalancerBootstrapConfig;
use paddler_bootstrap::balancer_service_bundle::BalancerServiceBundle;
use paddler_cluster_harness::ephemeral_loopback_addr::ephemeral_loopback_addr;

fn fully_configured_runner_config() -> BalancerBootstrapConfig {
    BalancerBootstrapConfig {
        buffered_request_timeout: Duration::from_secs(10),
        inference_service_configuration: InferenceServiceConfiguration {
            addr: ResolvedSocketAddr::from(ephemeral_loopback_addr()),
            cors_allowed_hosts: vec![],
            inference_item_timeout: Duration::from_secs(30),
        },
        management_service_configuration: ManagementServiceConfiguration {
            addr: ResolvedSocketAddr::from(ephemeral_loopback_addr()),
            cors_allowed_hosts: vec![],
        },
        max_buffered_requests: 30,
        openai_service_configuration: Some(CompatibilityServiceConfiguration {
            addr: ResolvedSocketAddr::from(ephemeral_loopback_addr()),
        }),
        state_database_type: StateDatabaseType::Memory(StoredBalancerDesiredState::NothingStored),
        statsd_prefix: "paddler_bootstrap_test_".to_owned(),
        statsd_service_configuration: Some(StatsdServiceConfiguration {
            statsd_addr: ResolvedSocketAddr::from(ephemeral_loopback_addr()),
            statsd_reporting_interval: Duration::from_secs(10),
        }),
        typesafe_service_configuration: Some(CompatibilityServiceConfiguration {
            addr: ResolvedSocketAddr::from(ephemeral_loopback_addr()),
        }),
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: Some(WebAdminPanelServiceConfiguration {
            addr: ephemeral_loopback_addr(),
        }),
    }
}

#[tokio::test]
async fn balancer_service_bundle_reports_the_bound_address_of_every_configured_service() {
    let bundle = BalancerServiceBundle::new(fully_configured_runner_config())
        .await
        .expect("a fully configured bundle must bind its ephemeral addresses");

    let bound_ports = [
        bundle
            .addresses
            .compat_openai
            .map(|compat_openai| compat_openai.port()),
        bundle
            .addresses
            .compat_typesafe
            .map(|compat_typesafe| compat_typesafe.port()),
        Some(bundle.addresses.inference.port()),
        Some(bundle.addresses.management.port()),
        #[cfg(feature = "web_admin_panel")]
        bundle
            .addresses
            .web_admin_panel
            .map(|web_admin_panel| web_admin_panel.port()),
    ];

    assert!(
        bound_ports
            .iter()
            .all(|bound_port| bound_port.is_some_and(|port| port != 0))
    );
}
