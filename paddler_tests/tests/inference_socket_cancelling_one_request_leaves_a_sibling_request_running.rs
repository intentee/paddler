#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_cancelling_one_request_leaves_a_sibling_request_running() {
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)])
        .await
        .expect("the cluster must start");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();

    let cancelled_request_token = CancellationToken::new();
    let kept_request_token = CancellationToken::new();

    let mut cancelled_stream = cluster
        .client_inference
        .continue_from_raw_prompt(
            cancelled_request_token.clone(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(500).unwrap(),
                raw_prompt: "Write a long story about an explorer".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    let kept_stream = cluster
        .client_inference
        .continue_from_raw_prompt(
            kept_request_token.clone(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(32).unwrap(),
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    cluster
        .wait_for_slots_processing(&agent_id, 2)
        .await
        .expect("both requests should occupy a slot");

    cancelled_stream
        .next()
        .await
        .expect("the cancelled request must produce at least one message first")
        .expect("the message must be readable");

    cancelled_request_token.cancel();

    assert!(
        cancelled_stream.next().await.is_none(),
        "the cancelled request must end its stream"
    );

    let kept_tokens = collect_generated_tokens(kept_stream)
        .await
        .expect("the generated tokens must be collected");

    assert!(
        !kept_tokens.token_results.is_empty(),
        "the sibling request sharing the same inference socket must keep generating tokens"
    );

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect("both slots should be released once the sibling request completes");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
