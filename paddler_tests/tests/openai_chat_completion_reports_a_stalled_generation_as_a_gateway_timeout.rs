#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use reqwest::StatusCode;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::openai_chat_completion_failure_status::openai_chat_completion_failure_status;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_reports_a_stalled_generation_as_a_gateway_timeout() {
    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_0_6b().into_desired_state())),
        inference_item_timeout: Duration::ZERO,
        ..ClusterParams::default()
    })
    .await
    .expect("a text generation cluster must start");

    assert_eq!(
        openai_chat_completion_failure_status(&cluster)
            .await
            .expect("the chat completion must fail with an OpenAI error body"),
        StatusCode::GATEWAY_TIMEOUT
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
