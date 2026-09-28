#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::load_qwen3_after_request_is_buffered::load_qwen3_after_request_is_buffered;
use paddler_tests::start_cluster_without_model::start_cluster_without_model;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_buffered_conversation_after_switching_to_embeddings() -> Result<()> {
    let mut cluster = start_cluster_without_model(
        AgentConfig::uniform(1, 1),
        InferenceParameters::deterministic(),
    )
    .await?;

    let request = cluster.continue_from_conversation_history(
        CancellationToken::new(),
        &ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("Hello".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 4,
            parse_tool_calls: false,
            tools: vec![],
        },
    );
    let collected = load_qwen3_after_request_is_buffered(&mut cluster, request, true).await?;

    assert_eq!(
        collected
            .token_results
            .into_iter()
            .map(|token_result_with_producer| token_result_with_producer.token_result)
            .collect::<Vec<GeneratedTokenResult>>(),
        vec![GeneratedTokenResult::TokenGenerationDisabled(
            "Some(\"test-agent-0\"): token generation is disabled because this agent is running in embeddings-only mode"
                .to_owned()
        )]
    );

    cluster.shutdown().await?;

    Ok(())
}
