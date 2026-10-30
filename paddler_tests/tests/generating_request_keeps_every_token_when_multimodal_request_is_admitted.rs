#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use futures_util::stream::iter;
use tokio::spawn;
use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::is_unending_generation_text::is_unending_generation_text;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;

#[tokio::test(flavor = "multi_thread")]
async fn generating_request_keeps_every_token_when_multimodal_request_is_admitted() {
    let cluster = start_cluster_with_smolvlm2(AgentConfig::uniform(1, 2))
        .await
        .expect("the cluster must start");
    let text_cancellation = CancellationToken::new();
    let mut text_stream = cluster
        .continue_from_raw_prompt_stream(text_cancellation.clone(), &unending_generation())
        .await
        .expect("the inference request must be accepted");

    let first_streamed_item = text_stream
        .next()
        .await
        .expect("the text request must stream a first token");
    let streamed_text = spawn(collect_generated_tokens(Box::pin(
        iter([first_streamed_item]).chain(text_stream),
    )));

    cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![
                    ConversationMessage::Message {
                        content: ConversationMessageContent::Parts(vec![
                            ConversationMessageContentPart::ImageUrl {
                                image_url: ImageUrl {
                                    url: load_test_image_data_uri()
                                        .expect("the test image must load"),
                                },
                            },
                            ConversationMessageContentPart::Text {
                                text: "Describe this image.".to_owned(),
                            },
                        ]),
                        role: "user".to_owned(),
                    },
                ]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::new(4).unwrap(),
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted")
        .summary()
        .expect("the generation must finish with a summary");

    text_cancellation.cancel();

    let streamed_text = streamed_text
        .await
        .expect("the text collection must not panic")
        .expect("the message must be readable")
        .text;

    assert!(
        is_unending_generation_text(&streamed_text),
        "{streamed_text:?}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
