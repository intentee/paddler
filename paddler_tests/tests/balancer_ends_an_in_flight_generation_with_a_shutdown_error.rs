#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio::spawn;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::inference_stream_error_codes::inference_stream_error_codes;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_ends_an_in_flight_generation_with_a_shutdown_error() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("a single-slot qwen3 cluster must start");
    let mut stream = cluster
        .client_inference
        .post_continue_from_raw_prompt(CancellationToken::new(), &unending_generation())
        .await
        .expect("the generation must start");

    stream
        .next()
        .await
        .expect("the generation must stream its first message")
        .expect("the first message must be readable");

    let Cluster {
        agents, balancer, ..
    } = cluster;
    let balancer_shutdown = spawn(balancer.shutdown());

    assert_eq!(
        inference_stream_error_codes(stream)
            .await
            .expect("every streamed message must be readable"),
        vec![503]
    );

    balancer_shutdown
        .await
        .expect("the balancer shutdown task must not panic")
        .expect("the balancer must shut down cleanly");

    for agent in agents {
        agent
            .shutdown()
            .await
            .expect("the agent must shut down cleanly");
    }
}
