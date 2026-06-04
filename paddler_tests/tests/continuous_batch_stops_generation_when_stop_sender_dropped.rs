#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_stops_generation_when_stop_sender_dropped() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)]).await?;

    let mut first_stream = cluster
        .continue_from_raw_prompt_stream(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 500,
            raw_prompt: "Write a long essay about photosynthesis".to_owned(),
        })
        .await?;

    let _first_token = first_stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("first stream must yield at least one message"))?;

    drop(first_stream);

    let second_collected = cluster
        .continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await?;

    assert!(matches!(
        second_collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    let second_token_count = second_collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(
        second_token_count > 0,
        "second sequential request must succeed after first stream is dropped"
    );

    cluster.shutdown().await?;

    Ok(())
}
