#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;

#[tokio::test(flavor = "multi_thread")]
async fn agent_refuses_an_image_data_uri_without_the_base64_marker() {
    let cluster = start_cluster_with_smolvlm2(AgentConfig::uniform(1, 2))
        .await
        .expect("the cluster must start");

    let outcome = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![
                    ConversationMessage::Message {
                        content: ConversationMessageContent::Parts(vec![
                            ConversationMessageContentPart::ImageUrl {
                                image_url: ImageUrl {
                                    url: "data:image/jpegbase64,abc123".to_owned(),
                                },
                            },
                            ConversationMessageContentPart::Text {
                                text: "Describe this image".to_owned(),
                            },
                        ]),
                        role: "user".to_owned(),
                    },
                ]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::new(20).unwrap(),
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await;

    let collected = outcome.expect("the request must complete");
    assert_eq!(
        collected.into_token_results(),
        vec![GeneratedTokenResult::ImageDecodingFailed(
            "test-agent-0: failed to decode images: Data URI metadata \"image/jpegbase64\" does not end with the ;base64 marker"
                .to_owned()
        )]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
