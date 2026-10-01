#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_releases_slots_on_shutdown_with_active_request() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let mut stream = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(500).unwrap(),
                raw_prompt: "Write a long essay".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    let _first_message = stream
        .next()
        .await
        .expect("inference stream must yield at least one message");

    drop(stream);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
