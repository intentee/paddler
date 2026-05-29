#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler::generated_token_result::GeneratedTokenResult;
use paddler::request_params::ContinueFromRawPromptParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::token_result_with_producer::TokenResultWithProducer;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_generates_tokens_from_raw_prompt() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let collected = cluster
        .continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 30,
            raw_prompt:
                "<|im_start|>user\nHow can I make a cat happy?<|im_end|>\n<|im_start|>assistant\n"
                    .to_owned(),
        })
        .await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(token_count > 0);
    assert!(matches!(
        collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    cluster.shutdown().await?;

    Ok(())
}
