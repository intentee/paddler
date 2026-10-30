use std::mem::discriminant;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_balancer::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;

use crate::running_balancer_subprocess::running_balancer_subprocess;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_without_compatibility_flags_announces_no_compatibility_addresses() {
    let running_balancer = running_balancer_subprocess().await;

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
