use std::mem::discriminant;

use paddler_balancer::constants::OPENAI_API_PATH_CHAT_COMPLETIONS;
use paddler_balancer::constants::TYPESAFE_API_PATH_SYSTEM_ONE;
use paddler_cli_tests::running_balancer_subprocess::running_balancer_subprocess;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_without_compatibility_flags_announces_no_compatibility_addresses() {
    let running_balancer = running_balancer_subprocess(env!("CARGO_BIN_EXE_paddler_cluster_node"))
        .await
        .expect("the balancer must start and announce its addresses");

    let compat_openai_url = running_balancer.compat_openai_url(OPENAI_API_PATH_CHAT_COMPLETIONS);
    let compat_typesafe_url = running_balancer.compat_typesafe_url(TYPESAFE_API_PATH_SYSTEM_ONE);

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
