#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler::conversation_history::ConversationHistory;
use paddler::conversation_message::ConversationMessage;
use paddler::conversation_message_content::ConversationMessageContent;
use paddler::generated_token_result::GeneratedTokenResult;
use paddler::grammar_constraint::GrammarConstraint;
use paddler::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_grammar_with_thinking_returns_incompatible_error() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(AgentConfig::single(1)).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let outcome = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("What is 2+2?".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: true,
            grammar: Some(GrammarConstraint::JsonSchema {
                schema: r#"{"type": "object", "properties": {"answer": {"type": "string"}}, "required": ["answer"]}"#.to_owned(),
            }),
            max_tokens: 50,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await;

    if let Ok(stream) = outcome {
        let collected = collect_generated_tokens(stream).await;
        if let Ok(collected) = collected {
            assert!(
                collected.token_results.iter().any(|result| matches!(
                    result.token_result,
                    GeneratedTokenResult::GrammarIncompatibleWithThinking(_)
                )),
                "expected GrammarIncompatibleWithThinking, got: {:?}",
                collected.token_results
            );
        }
    }

    cluster.shutdown().await?;

    Ok(())
}
