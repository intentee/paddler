#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_cli_tests::inference_http_client::InferenceHttpClient;
use paddler_cli_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_internal_endpoint_pure_content_usage_breakdown() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(AgentConfig::single(1)).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("Say hello.".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 60,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let last = collected
        .token_results
        .last()
        .ok_or_else(|| anyhow::anyhow!("no token results received"))?;
    let GeneratedTokenResult::Done(summary) = &last.token_result else {
        anyhow::bail!("last result was not Done: {last:?}");
    };

    assert!(summary.usage.prompt_tokens > 0);
    assert!(summary.usage.content_tokens > 0);
    assert_eq!(summary.usage.reasoning_tokens, 0);
    assert_eq!(summary.usage.tool_call_tokens, 0);
    assert_eq!(
        summary.usage.completion_tokens(),
        summary.usage.content_tokens + summary.usage.undeterminable_tokens
    );

    cluster.shutdown().await?;

    Ok(())
}
