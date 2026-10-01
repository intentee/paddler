#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_cancellation_releases_the_agent_slot() {
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();

    let cancellation_token = CancellationToken::new();

    let mut stream = cluster
        .client_inference
        .continue_from_raw_prompt(cancellation_token.clone(), unending_generation())
        .await
        .expect("the inference request must be accepted");

    stream
        .next()
        .await
        .expect("inference stream must produce at least one message")
        .expect("the message must be readable");

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .expect("the request should occupy the only slot");

    cancellation_token.cancel();

    assert!(
        stream.next().await.is_none(),
        "a cancelled inference socket request must end its stream"
    );

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect("the slot should be released after the request is cancelled");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
