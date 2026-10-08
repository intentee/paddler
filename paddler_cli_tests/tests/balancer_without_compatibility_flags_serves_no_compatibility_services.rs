use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_without_compatibility_flags_serves_no_compatibility_services() {
    let ephemeral_loopback_addr = EPHEMERAL_LOOPBACK_ADDR.to_string();
    let running_balancer = spawn_balancer_subprocess(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        [
            "--inference-addr",
            &ephemeral_loopback_addr,
            "--management-addr",
            &ephemeral_loopback_addr,
        ],
    )
    .await
    .expect("the balancer must start and announce its addresses");

    let compat_openai_addr = running_balancer.compat_openai_addr();
    let compat_typesafe_addr = running_balancer.compat_typesafe_addr();

    running_balancer
        .shutdown()
        .await
        .expect("the balancer subprocess must shut down");

    assert!(matches!(
        compat_openai_addr,
        Err(ClusterHarnessError::CompatOpenAIServiceNotServed)
    ));
    assert!(matches!(
        compat_typesafe_addr,
        Err(ClusterHarnessError::CompatTypeSafeServiceNotServed)
    ));
}
