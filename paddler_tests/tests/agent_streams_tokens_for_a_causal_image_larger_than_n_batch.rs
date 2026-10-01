#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::load_fixture_data_uri::load_fixture_data_uri;
use paddler_tests::start_cluster_with_smolvlm2_and_n_batch::start_cluster_with_smolvlm2_and_n_batch;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(20).unwrap();
const N_BATCH_SMALLER_THAN_ONE_IMAGE: u32 = 32;

async fn assert_streams_tokens_for_image(
    cluster: &Cluster,
    fixture_name: &str,
    mime_type: &str,
) -> Result<()> {
    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Parts(vec![
                        ConversationMessageContentPart::ImageUrl {
                            image_url: ImageUrl {
                                url: load_fixture_data_uri(fixture_name, mime_type)?,
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

    assert!(
        collected
            .token_results
            .iter()
            .any(|token_result_with_producer| token_result_with_producer.token_result.is_token()),
        "fixture {fixture_name} must stream tokens: {:?}",
        collected.token_results
    );
    assert!(
        collected.summary()?.usage.input_image_tokens > u64::from(N_BATCH_SMALLER_THAN_ONE_IMAGE)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_streams_tokens_for_a_causal_image_larger_than_n_batch() {
    let cluster = start_cluster_with_smolvlm2_and_n_batch(
        vec![AgentConfig::single(1)],
        N_BATCH_SMALLER_THAN_ONE_IMAGE,
    )
    .await
    .expect("the cluster must start");

    assert_streams_tokens_for_image(&cluster, "sarnow.jpeg", "image/jpeg")
        .await
        .expect("the image request must stream tokens");
    assert_streams_tokens_for_image(&cluster, "llamas.webp", "image/webp")
        .await
        .expect("the image request must stream tokens");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
