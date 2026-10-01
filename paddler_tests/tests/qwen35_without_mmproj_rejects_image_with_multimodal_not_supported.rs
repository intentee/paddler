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
use paddler_tests::start_cluster_with_qwen3_5::start_cluster_with_qwen3_5;

#[tokio::test(flavor = "multi_thread")]
async fn qwen35_without_mmproj_rejects_image_with_multimodal_not_supported() {
    let cluster = start_cluster_with_qwen3_5(vec![AgentConfig::single(1)], false)
        .await
        .expect("the cluster must start");

    let image_data_uri = load_test_image_data_uri().expect("the test image must load");

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
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history,
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::new(100).unwrap(),
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await;

    let collected = collected.expect("the request must complete");
    assert!(
        collected.token_results.iter().any(|result| matches!(
            result.token_result,
            GeneratedTokenResult::MultimodalNotSupported(_)
        )),
        "expected MultimodalNotSupported, got: {:?}",
        collected.token_results
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
