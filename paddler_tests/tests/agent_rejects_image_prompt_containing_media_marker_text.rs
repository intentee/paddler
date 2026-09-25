#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::mtmd::mtmd_default_marker;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_cluster_with_qwen2_5_vl::start_cluster_with_qwen2_5_vl;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_image_prompt_containing_media_marker_text() -> Result<()> {
    let cluster = start_cluster_with_qwen2_5_vl(vec![AgentConfig::single(1)]).await?;

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
                            text: format!("What does {} mean?", mtmd_default_marker()?),
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

    assert_eq!(
        collected
            .token_results
            .into_iter()
            .map(|result| result.token_result)
            .collect::<Vec<GeneratedTokenResult>>(),
        vec![GeneratedTokenResult::SamplerError(
            "Some(\"test-agent\"): failed to tokenize multimodal input: media preprocessing failed (image or audio)"
                .to_owned(),
        )]
    );

    cluster.shutdown().await?;

    Ok(())
}
