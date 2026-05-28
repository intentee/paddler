#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_cluster_with_qwen3_5::start_cluster_with_qwen3_5;
use paddler::conversation_history::ConversationHistory;
use paddler::conversation_message::ConversationMessage;
use paddler::conversation_message_content::ConversationMessageContent;
use paddler::generated_token_result::GeneratedTokenResult;
use paddler::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen35_internal_endpoint_with_thinking_enabled_emits_reasoning_tokens() -> Result<()> {
    let cluster = start_cluster_with_qwen3_5(vec![AgentConfig::single(1)], false).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text(
                    "What is two plus two? Think step by step.".to_owned(),
                ),
                role: "user".to_owned(),
            }]),
            enable_thinking: true,
            grammar: None,
            max_tokens: 600,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let reasoning_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result.token_result, GeneratedTokenResult::ReasoningToken(_)))
        .count();

    assert!(
        reasoning_count > 0,
        "Qwen3.5: expected at least one reasoning token when thinking is enabled (got {reasoning_count})"
    );

    let last = collected
        .token_results
        .last()
        .ok_or_else(|| anyhow::anyhow!("no token results received"))?;
    let GeneratedTokenResult::Done(summary) = &last.token_result else {
        anyhow::bail!("last result was not Done: {last:?}");
    };

    assert!(summary.usage.prompt_tokens > 0);
    assert!(summary.usage.reasoning_tokens > 0);
    assert_eq!(
        summary.usage.completion_tokens(),
        summary.usage.content_tokens
            + summary.usage.reasoning_tokens
            + summary.usage.undeterminable_tokens
    );

    let reasoning_stream: String = collected
        .token_results
        .iter()
        .filter_map(|result| match &result.token_result {
            GeneratedTokenResult::ReasoningToken(piece) => Some(piece.as_str()),
            _ => None,
        })
        .collect();
    let content_stream: String = collected
        .token_results
        .iter()
        .filter_map(|result| match &result.token_result {
            GeneratedTokenResult::ContentToken(piece) => Some(piece.as_str()),
            _ => None,
        })
        .collect();

    for forbidden in ["<think>", "</think>"] {
        assert!(
            !reasoning_stream.contains(forbidden),
            "Qwen3.5: reasoning stream leaked marker {forbidden:?}; \
             reasoning_stream={reasoning_stream:?}"
        );
        assert!(
            !content_stream.contains(forbidden),
            "Qwen3.5: content stream leaked marker {forbidden:?}; \
             content_stream={content_stream:?}"
        );
    }

    cluster.shutdown().await?;

    Ok(())
}
