#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_grammar_with_thinking_returns_incompatible_error() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let outcome = cluster
        .continue_from_conversation_history(CancellationToken::new(), &ContinueFromConversationHistoryParams {
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

    if let Ok(collected) = outcome {
        assert!(
            collected.token_results.iter().any(|result| matches!(
                result.token_result,
                GeneratedTokenResult::GrammarIncompatibleWithThinking(_)
            )),
            "expected GrammarIncompatibleWithThinking, got: {:?}",
            collected.token_results
        );
    }

    cluster.shutdown().await?;

    Ok(())
}
