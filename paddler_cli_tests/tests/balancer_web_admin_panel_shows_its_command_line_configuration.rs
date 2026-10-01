use std::collections::BTreeMap;
use std::process::Stdio;

use reqwest::get;
use tokio::net::UdpSocket;

use paddler_cli_tests::paddler_command::paddler_command;
use paddler_cli_tests::read_balancer_addresses::read_balancer_addresses;
use paddler_cli_tests::subprocess_process::SubprocessProcess;
use paddler_test_cluster_harness::dashboard_attributes::dashboard_attributes;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;
use paddler_test_cluster_harness::managed_process::ManagedProcess as _;

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
    let mut balancer_subprocess = paddler_command(env!("CARGO_BIN_EXE_paddler_cluster_node"))
        .args([
            "balancer",
            "--buffered-request-timeout",
            "1500",
            "--compat-openai-addr",
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
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the balancer subprocess must spawn");
    let balancer_stdout = balancer_subprocess
        .stdout
        .take()
        .expect("the balancer stdout must be piped");
    let balancer_process = Box::new(SubprocessProcess::new(balancer_subprocess));
    let addresses = read_balancer_addresses(balancer_stdout)
        .await
        .expect("the balancer must announce its addresses");
    let web_admin_panel_addr = addresses
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

    balancer_process
        .shutdown()
        .await
        .expect("the balancer subprocess must shut down");

    assert_eq!(
        dashboard_attributes(&dashboard).expect("the dashboard must carry its configuration"),
        BTreeMap::from(
            [
                ("data-buffered-request-timeout-millis", "1500"),
                ("data-compat-openai-addr", ephemeral_loopback_addr.as_str()),
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
