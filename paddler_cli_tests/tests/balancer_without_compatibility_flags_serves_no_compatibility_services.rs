use std::mem::discriminant;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_balancer::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use paddler_cli_tests::signalled_subprocess::SignalledSubprocess;
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
        SignalledSubprocess::new,
    )
    .await
    .expect("the balancer must start and announce its addresses")
    .running_balancer;

    let compat_openai_url = running_balancer.compat_openai_url(OpenAIApiPath::CHAT_COMPLETIONS);
    let compat_typesafe_url = running_balancer.compat_typesafe_url(TypeSafeApiPath::SYSTEM_ONE);

    running_balancer
        .shutdown()
        .await
        .expect("the balancer subprocess must shut down");

    assert_eq!(
        compat_openai_url.err().as_ref().map(discriminant),
        Some(discriminant(
            &ClusterHarnessError::CompatOpenAIServiceNotServed
        ))
    );
    assert_eq!(
        compat_typesafe_url.err().as_ref().map(discriminant),
        Some(discriminant(
            &ClusterHarnessError::CompatTypeSafeServiceNotServed
        ))
    );
}
