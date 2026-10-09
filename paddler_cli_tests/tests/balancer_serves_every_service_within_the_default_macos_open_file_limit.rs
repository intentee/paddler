use nix::sys::resource::Resource;
use nix::sys::resource::getrlimit;
use nix::sys::resource::rlim_t;
use nix::sys::resource::setrlimit;
use reqwest::StatusCode;
use reqwest::get;
use tokio_util::sync::CancellationToken;

use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_client::client_health::ClientHealth;
use paddler_client::reports_health::ReportsHealth as _;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

const MACOS_DEFAULT_SOFT_OPEN_FILE_LIMIT: rlim_t = 256;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_every_service_within_the_default_macos_open_file_limit() {
    let (_, hard_open_file_limit) =
        getrlimit(Resource::RLIMIT_NOFILE).expect("the open file limit must be readable");

    setrlimit(
        Resource::RLIMIT_NOFILE,
        MACOS_DEFAULT_SOFT_OPEN_FILE_LIMIT,
        hard_open_file_limit,
    )
    .expect("the soft open file limit must be lowerable");

    let ephemeral_loopback_addr = EPHEMERAL_LOOPBACK_ADDR.to_string();
    let running_balancer = spawn_balancer_subprocess(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        [
            "--compat-openai-addr",
            &ephemeral_loopback_addr,
            "--compat-typesafe-addr",
            &ephemeral_loopback_addr,
            "--inference-addr",
            &ephemeral_loopback_addr,
            "--management-addr",
            &ephemeral_loopback_addr,
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

    let health_base_urls = [
        running_balancer
            .inference_base_url()
            .expect("the inference address must form a base URL"),
        running_balancer
            .management_base_url()
            .expect("the management address must form a base URL"),
        running_balancer
            .compat_openai_base_url()
            .expect("the OpenAI compatibility address must form a base URL"),
        running_balancer
            .compat_typesafe_base_url()
            .expect("the TypeSafe compatibility address must form a base URL"),
    ];
    let mut health_checks = Vec::new();

    for health_base_url in health_base_urls {
        health_checks.push(
            ClientHealth::new(health_base_url)
                .get_health(CancellationToken::new())
                .await
                .expect("every balancer service must serve health checks"),
        );
    }

    let dashboard_status = get(format!("http://{web_admin_panel_addr}/"))
        .await
        .expect("the web admin panel must accept connections")
        .status();

    running_balancer
        .shutdown()
        .await
        .expect("the balancer subprocess must shut down");

    assert_eq!(health_checks, ["OK", "OK", "OK", "OK"]);
    assert_eq!(dashboard_status, StatusCode::OK);
}
