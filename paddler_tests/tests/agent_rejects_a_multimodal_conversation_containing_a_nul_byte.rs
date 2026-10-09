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
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_a_multimodal_conversation_containing_a_nul_byte() {
    let cluster = start_cluster_with_smolvlm2(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Parts(vec![
                        ConversationMessageContentPart::ImageUrl {
                            image_url: ImageUrl {
                                url: load_test_image_data_uri().expect("the test image must load"),
                            },
                        },
                        ConversationMessageContentPart::Text {
                            text: "before\0after".to_owned(),
                        },
                    ]),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert!(matches!(
        collected.into_token_results().as_slice(),
        [GeneratedTokenResult::PromptTokenizationFailed(description)]
            if !description.is_empty()
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
