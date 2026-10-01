#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_releases_slot_when_client_disconnects() {
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();

    let mut stream = cluster
        .continue_from_raw_prompt_stream(CancellationToken::new(), &unending_generation())
        .await
        .expect("the inference request must be accepted");

    let first_message = stream
        .next()
        .await
        .expect("inference stream must produce at least one message");

    drop(first_message);

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .expect("first request should occupy the only slot");

    drop(stream);

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect("slot should be released after the HTTP client disconnects");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
