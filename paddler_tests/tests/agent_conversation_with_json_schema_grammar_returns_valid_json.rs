#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler::conversation_history::ConversationHistory;
use paddler::conversation_message::ConversationMessage;
use paddler::conversation_message_content::ConversationMessageContent;
use paddler::grammar_constraint::GrammarConstraint;
use paddler::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_conversation_with_json_schema_grammar_returns_valid_json() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let collected = cluster
        .continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("What is 2+2?".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: Some(GrammarConstraint::JsonSchema {
                schema: r#"{"type": "object", "properties": {"answer": {"type": "string"}}, "required": ["answer"]}"#.to_owned(),
            }),
            max_tokens: 50,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&collected.text)?;

    assert!(
        parsed.get("answer").is_some(),
        "JSON schema grammar should produce JSON with 'answer' field; got {:?}",
        collected.text
    );

    cluster.shutdown().await?;

    Ok(())
}
