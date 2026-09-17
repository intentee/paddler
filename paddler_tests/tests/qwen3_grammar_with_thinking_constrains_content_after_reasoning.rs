#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use anyhow::anyhow;
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
async fn qwen3_grammar_with_thinking_constrains_content_after_reasoning() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let collected = cluster
        .continue_from_conversation_history(CancellationToken::new(), &ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text(
                    "What is 2+2? Think briefly, then answer.".to_owned(),
                ),
                role: "user".to_owned(),
            }]),
            enable_thinking: true,
            grammar: Some(GrammarConstraint::JsonSchema {
                schema: r#"{"type": "object", "properties": {"answer": {"type": "string"}}, "required": ["answer"]}"#.to_owned(),
            }),
            max_tokens: 600,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let reasoning_text: String = collected
        .token_results
        .iter()
        .filter_map(|result| match &result.token_result {
            GeneratedTokenResult::ReasoningToken(token_text) => Some(token_text.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        !reasoning_text.is_empty(),
        "thinking must still produce reasoning tokens when a grammar is attached"
    );

    let content_text: String = collected
        .token_results
        .iter()
        .filter_map(|result| match &result.token_result {
            GeneratedTokenResult::ContentToken(token_text) => Some(token_text.as_str()),
            _ => None,
        })
        .collect();

    let parsed: serde_json::Value = serde_json::from_str(&content_text).map_err(|err| {
        anyhow!("content after reasoning must satisfy the grammar, got {content_text:?}: {err}")
    })?;

    assert!(
        parsed.get("answer").is_some(),
        "JSON schema grammar should produce JSON with an 'answer' field; got {content_text:?}"
    );

    cluster.shutdown().await?;

    Ok(())
}
