#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_raw_prompt_containing_nul_byte() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(20).unwrap(),
                raw_prompt: "before\0after".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert_eq!(
        collected.into_token_results(),
        vec![GeneratedTokenResult::PromptTokenizationFailed(
            "test-agent: failed to tokenize prompt: nul byte found in provided data at position: 6"
                .to_owned(),
        )]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
