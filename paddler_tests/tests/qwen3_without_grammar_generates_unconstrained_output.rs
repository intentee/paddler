#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::request_params::ContinueFromRawPromptParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_without_grammar_generates_unconstrained_output() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let collected = cluster
        .continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 20,
            raw_prompt: "<|im_start|>user\nSay hello<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n".to_owned(),
        })
        .await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(token_count > 0);

    cluster.shutdown().await?;

    Ok(())
}
