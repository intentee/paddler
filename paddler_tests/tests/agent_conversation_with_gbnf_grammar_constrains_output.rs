#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_conversation_with_gbnf_grammar_constrains_output() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "Is the sky blue? Answer yes or no.".to_owned(),
                    ),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: Some(GrammarConstraint::Gbnf {
                    grammar: r"root ::= [Yy][Ee][Ss] | [Nn][Oo]".to_owned(),
                    root: "root".to_owned(),
                }),
                max_tokens: 10,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await?;

    let lower = collected.text.to_lowercase();

    assert!(
        lower == "yes" || lower == "no",
        "GBNF grammar should constrain output to yes/no; got {:?}",
        collected.text
    );

    cluster.shutdown().await?;

    Ok(())
}
