#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn management_agents_stream_yields_initial_snapshot() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2))
        .await
        .expect("the cluster must start");

    let mut stream = cluster
        .client_management
        .get_agents_stream(CancellationToken::new())
        .await
        .expect("agents stream should connect");

    let first_event = stream
        .next()
        .await
        .expect("agents stream must produce at least one event")
        .expect("first agents stream event should deserialize");

    assert!(
        !first_event.agents.is_empty(),
        "first agents stream event must contain at least one agent"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
