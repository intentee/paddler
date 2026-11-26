#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_request_params_fixture::sample_decision::sample_decision;

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_whose_agents_only_generate_text_refuses_decisions_as_unavailable() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("a cluster serving a text generation model must start");

    let refusal = cluster
        .client_inference
        .post_decide(CancellationToken::new(), &sample_decision())
        .await
        .err()
        .expect("a cluster that only generates text must refuse decisions");

    assert!(matches!(refusal, ClientError::ServiceUnavailable { .. }));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
