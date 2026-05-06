#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3_6::start_in_process_cluster_with_qwen3_6;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen36_internal_endpoint_with_thinking_disabled_emits_only_content_tokens() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3_6(1).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("What is two plus two?".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 200,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let reasoning_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::ReasoningToken(_)))
        .count();
    let content_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::ContentToken(_)))
        .count();
    let undeterminable_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::UndeterminableToken(_)))
        .count();

    assert_eq!(
        reasoning_count, 0,
        "Qwen3.6 thinking-disabled: classifier must not stream any reasoning tokens \
         (got reasoning_count={reasoning_count}, content_count={content_count}, \
         undeterminable_count={undeterminable_count})"
    );
    assert_eq!(
        undeterminable_count, 0,
        "Qwen3.6 thinking-disabled: prompt-token replay must move section to Content \
         before generation, so no UndeterminableToken may stream; \
         (got reasoning_count={reasoning_count}, content_count={content_count}, \
         undeterminable_count={undeterminable_count})"
    );
    assert!(
        content_count > 0,
        "Qwen3.6 thinking-disabled: classifier must stream at least one content token"
    );

    let last = collected
        .token_results
        .last()
        .ok_or_else(|| anyhow::anyhow!("no token results received"))?;
    let GeneratedTokenResult::Done(summary) = last else {
        anyhow::bail!("last result was not Done: {last:?}");
    };

    assert!(summary.usage.prompt_tokens > 0);
    assert_eq!(
        summary.usage.reasoning_tokens, 0,
        "Qwen3.6 thinking-disabled: usage.reasoning_tokens must be zero; usage={:?}",
        summary.usage
    );
    assert_eq!(
        summary.usage.undeterminable_tokens, 0,
        "Qwen3.6 thinking-disabled: usage.undeterminable_tokens must be zero; usage={:?}",
        summary.usage
    );
    assert_eq!(
        summary.usage.completion_tokens(),
        summary.usage.content_tokens,
        "Qwen3.6 thinking-disabled: completion tokens equal content tokens since \
         reasoning and undeterminable are zero"
    );

    let reasoning_stream: String = collected
        .token_results
        .iter()
        .filter_map(|result| match result {
            GeneratedTokenResult::ReasoningToken(piece) => Some(piece.as_str()),
            _ => None,
        })
        .collect();
    let content_stream: String = collected
        .token_results
        .iter()
        .filter_map(|result| match result {
            GeneratedTokenResult::ContentToken(piece) => Some(piece.as_str()),
            _ => None,
        })
        .collect();

    for forbidden in ["<think>", "</think>"] {
        assert!(
            !reasoning_stream.contains(forbidden),
            "Qwen3.6 thinking-disabled: reasoning stream leaked marker {forbidden:?}; \
             reasoning_stream={reasoning_stream:?}"
        );
        assert!(
            !content_stream.contains(forbidden),
            "Qwen3.6 thinking-disabled: content stream leaked marker {forbidden:?}; \
             content_stream={content_stream:?}"
        );
    }

    cluster.shutdown().await?;

    Ok(())
}
