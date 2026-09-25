#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_cluster_with_qwen2_5_vl_and_context_size::start_cluster_with_qwen2_5_vl_and_context_size;
use tokio_util::sync::CancellationToken;

const SEQUENCE_CONTEXT_SIZE: u32 = 256;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_image_prompt_exceeding_sequence_context() -> Result<()> {
    let cluster = start_cluster_with_qwen2_5_vl_and_context_size(
        vec![AgentConfig::single(1)],
        SEQUENCE_CONTEXT_SIZE,
    )
    .await?;

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Parts(vec![
                        ConversationMessageContentPart::ImageUrl {
                            image_url: ImageUrl {
                                url: load_test_image_data_uri()?,
                            },
                        },
                        ConversationMessageContentPart::Text {
                            text: "Describe this image.".to_owned(),
                        },
                    ]),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: 20,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await?;

    assert!(collected.token_results.iter().any(|result| matches!(
        &result.token_result,
        GeneratedTokenResult::PromptExceedsContextSize(details)
            if details.sequence_context_size == SEQUENCE_CONTEXT_SIZE
                && details.prompt_tokens > SEQUENCE_CONTEXT_SIZE as usize
    )));

    cluster.shutdown().await?;

    Ok(())
}
