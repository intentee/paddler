#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use futures_util::future;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_cli_tests::inference_http_client::InferenceHttpClient;
use paddler_cli_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::generation_summary::GenerationSummary;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_internal_endpoint_concurrent_requests_keep_independent_usage() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(AgentConfig::single(2)).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let prompts = ["Say hi.", "Count to three."];

    let futures = prompts.iter().map(|prompt| {
        let client = inference_client.clone();
        let prompt = (*prompt).to_owned();
        async move {
            let stream = client
                .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
                    add_generation_prompt: true,
                    conversation_history: ConversationHistory::new(vec![ConversationMessage {
                        content: ConversationMessageContent::Text(prompt),
                        role: "user".to_owned(),
                    }]),
                    enable_thinking: false,
                    grammar: None,
                    max_tokens: 30,
                    parse_tool_calls: false,
                    tools: vec![],
                })
                .await?;

            let collected = collect_generated_tokens(stream).await?;

            let last = collected
                .token_results
                .last()
                .ok_or_else(|| anyhow::anyhow!("no token results received"))?;
            match &last.token_result {
                GeneratedTokenResult::Done(summary) => {
                    Ok::<GenerationSummary, anyhow::Error>(*summary)
                }
                other => Err(anyhow::anyhow!("last result was not Done: {other:?}")),
            }
        }
    });

    let summaries: Vec<GenerationSummary> = future::try_join_all(futures).await?;

    assert_eq!(summaries.len(), 2);

    for summary in &summaries {
        assert!(summary.usage.prompt_tokens > 0);
        assert!(summary.usage.completion_tokens() > 0);
    }

    // The two requests have different prompts and different generations;
    // their usage breakdowns must not be byte-identical.
    assert_ne!(
        summaries[0].usage, summaries[1].usage,
        "concurrent requests reported identical usage; counters likely shared"
    );

    cluster.shutdown().await?;

    Ok(())
}
