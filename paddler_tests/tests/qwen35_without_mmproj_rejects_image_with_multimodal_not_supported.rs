#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler::conversation_history::ConversationHistory;
use paddler::conversation_message::ConversationMessage;
use paddler::conversation_message_content::ConversationMessageContent;
use paddler::conversation_message_content_part::ConversationMessageContentPart;
use paddler::generated_token_result::GeneratedTokenResult;
use paddler::image_url::ImageUrl;
use paddler::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_cluster_with_qwen3_5::start_cluster_with_qwen3_5;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen35_without_mmproj_rejects_image_with_multimodal_not_supported() -> Result<()> {
    let cluster = start_cluster_with_qwen3_5(vec![AgentConfig::single(1)], false).await?;

    let image_data_uri = load_test_image_data_uri()?;

    let conversation_history = ConversationHistory::new(vec![ConversationMessage {
        content: ConversationMessageContent::Parts(vec![
            ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: image_data_uri,
                },
            },
            ConversationMessageContentPart::Text {
                text: "What do you see?".to_owned(),
            },
        ]),
        role: "user".to_owned(),
    }]);

    let collected = cluster
        .continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: false,
            grammar: None,
            max_tokens: 100,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await;

    if let Ok(collected) = collected {
        assert!(
            collected.token_results.iter().any(|result| matches!(
                result.token_result,
                GeneratedTokenResult::MultimodalNotSupported(_)
            )),
            "expected MultimodalNotSupported, got: {:?}",
            collected.token_results
        );
    }

    cluster.shutdown().await?;

    Ok(())
}
