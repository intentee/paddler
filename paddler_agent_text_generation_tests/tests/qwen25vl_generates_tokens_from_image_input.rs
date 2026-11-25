#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation_tests::start_cluster_with_qwen2_5_vl::start_cluster_with_qwen2_5_vl;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_file_fixture::constants::LLAMAS_JPEG;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;

#[tokio::test(flavor = "multi_thread")]
async fn qwen25vl_generates_tokens_from_image_input() {
    let cluster = start_cluster_with_qwen2_5_vl(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let image_data_uri = LLAMAS_JPEG.data_uri().expect("the test image must load");

    let conversation_history = ConversationHistory::new(vec![ConversationMessage::Message {
        content: ConversationMessageContent::Parts(vec![
            ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: image_data_uri,
                },
            },
            ConversationMessageContentPart::Text {
                text: "What do you see in this image?".to_owned(),
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
                enable_thinking: ModelDefaultOr::Explicit(false),
                grammar: None,
                max_tokens: NonZeroU32::new(200).unwrap(),
                parallel_tool_calls: ModelDefaultOr::ModelDefault,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    let token_count = collected
        .token_results
        .iter()
        .filter(|TokenResultWithProducer { token_result, .. }| token_result.is_token())
        .count();

    assert!(token_count > 0);
    collected
        .summary()
        .expect("the generation must finish with a summary");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
