use std::collections::BTreeMap;

use reqwest::get;
use tokio::net::UdpSocket;

use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_test_cluster_harness::dashboard_attributes::dashboard_attributes;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_web_admin_panel_shows_its_command_line_configuration() {
    let statsd_socket = UdpSocket::bind(EPHEMERAL_LOOPBACK_ADDR)
        .await
        .expect("the statsd socket must bind");
    let statsd_addr = statsd_socket
        .local_addr()
        .expect("the statsd socket must report its address")
        .to_string();
    let ephemeral_loopback_addr = EPHEMERAL_LOOPBACK_ADDR.to_string();
    let running_balancer = spawn_balancer_subprocess(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        [
            "--buffered-request-timeout",
            "1500",
            "--compat-openai-addr",
            &ephemeral_loopback_addr,
            "--compat-typesafe-addr",
            &ephemeral_loopback_addr,
            "--inference-addr",
            &ephemeral_loopback_addr,
            "--management-addr",
            &ephemeral_loopback_addr,
            "--max-buffered-requests",
            "7",
            "--statsd-addr",
            &statsd_addr,
            "--statsd-prefix",
            "dashboard_",
            "--statsd-reporting-interval",
            "3600000",
            "--web-admin-panel-addr",
            &ephemeral_loopback_addr,
        ],
    )
    .await
    .expect("the balancer must start and announce its addresses")
    .running_balancer;
    let web_admin_panel_addr = running_balancer
        .addresses
        .web_admin_panel
        .expect("the balancer must serve the web admin panel");
    let dashboard = get(format!("http://{web_admin_panel_addr}/"))
        .await
        .expect("the web admin panel must accept connections")
        .error_for_status()
        .expect("the web admin panel must render the dashboard")
        .text()
        .await
        .expect("the dashboard must have a body");

    running_balancer
        .shutdown()
        .await
        .expect("the balancer subprocess must shut down");

    assert_eq!(
        dashboard_attributes(&dashboard).expect("the dashboard must carry its configuration"),
        BTreeMap::from(
            [
                ("data-buffered-request-timeout-millis", "1500"),
                ("data-compat-openai-addr", ephemeral_loopback_addr.as_str()),
                (
                    "data-compat-typesafe-addr",
                    ephemeral_loopback_addr.as_str()
                ),
                ("data-inference-addr", ephemeral_loopback_addr.as_str()),
                ("data-management-addr", ephemeral_loopback_addr.as_str()),
                ("data-max-buffered-requests", "7"),
                ("data-statsd-addr", statsd_addr.as_str()),
                ("data-statsd-prefix", "dashboard_"),
                ("data-statsd-reporting-interval-millis", "3600000"),
                ("id", "paddler-dashboard"),
            ]
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
        )
    );
}
