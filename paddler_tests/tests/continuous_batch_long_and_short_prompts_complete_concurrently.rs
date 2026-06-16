#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_client::token_result_with_producer::TokenResultWithProducer;
use paddler_cluster::agent_config::AgentConfig;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_long_and_short_prompts_complete_concurrently() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)]).await?;

    let long_prompt = "Photosynthesis is the process by which green plants and certain other organisms transform light energy into chemical energy. During photosynthesis in green plants, light energy is captured and used to convert water, carbon dioxide, and minerals into oxygen and energy-rich organic compounds. Explain the process in detail:".to_owned();

    let long_params = ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 20,
        raw_prompt: long_prompt,
    };
    let short_params = ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 20,
        raw_prompt: "Hi".to_owned(),
    };
    let (long_collected, short_collected) = tokio::join!(
        cluster
            .inference_client
            .http()
            .continue_from_raw_prompt_collected(&long_params),
        cluster
            .inference_client
            .http()
            .continue_from_raw_prompt_collected(&short_params),
    );

    let long_collected = long_collected?;
    let short_collected = short_collected?;

    let long_tokens = long_collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();
    let short_tokens = short_collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(long_tokens > 0);
    assert!(short_tokens > 0);
    assert!(matches!(
        long_collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));
    assert!(matches!(
        short_collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    cluster.shutdown().await?;

    Ok(())
}
