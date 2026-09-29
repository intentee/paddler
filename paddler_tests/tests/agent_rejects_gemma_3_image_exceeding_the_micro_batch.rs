#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::oversized_media_details::OversizedMediaDetails;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::load_fixture_data_uri::load_fixture_data_uri;
use paddler_tests::start_cluster_with_gemma_3_and_mmproj_and_n_batch::start_cluster_with_gemma_3_and_mmproj_and_n_batch;
use tokio_util::sync::CancellationToken;

const GEMMA_3_IMAGE_TOKENS: usize = 256;
const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(20).unwrap();
const N_BATCH_SMALLER_THAN_ONE_IMAGE: u32 = 128;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_gemma_3_image_exceeding_the_micro_batch() -> Result<()> {
    let cluster = start_cluster_with_gemma_3_and_mmproj_and_n_batch(
        vec![AgentConfig::single(1)],
        N_BATCH_SMALLER_THAN_ONE_IMAGE,
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
                                url: load_fixture_data_uri("sarnow.jpeg", "image/jpeg")?,
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
                max_tokens: MAX_TOKENS,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await?;

    let token_results = collected.into_token_results();

    assert_eq!(
        token_results,
        vec![GeneratedTokenResult::MediaExceedsMicroBatch(
            OversizedMediaDetails {
                media_tokens: GEMMA_3_IMAGE_TOKENS,
                micro_batch_tokens: N_BATCH_SMALLER_THAN_ONE_IMAGE,
            }
        )]
    );

    cluster.shutdown().await?;

    Ok(())
}
