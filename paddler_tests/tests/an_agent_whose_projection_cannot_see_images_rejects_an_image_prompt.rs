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
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_test_cluster_harness::model_card::ultravox_v0_5_llama_3_2_1b::ultravox_v0_5_llama_3_2_1b;
use paddler_test_cluster_harness::model_card::ultravox_v0_5_llama_3_2_1b_mmproj::ultravox_v0_5_llama_3_2_1b_mmproj;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_whose_projection_cannot_see_images_rejects_an_image_prompt() {
    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: ClusterDesiredState::Apply(Box::new(
            ultravox_v0_5_llama_3_2_1b()
                .into_desired_state_with_multimodal_projection(ultravox_v0_5_llama_3_2_1b_mmproj()),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("an agent with an audio projection must start");

    let collected = cluster
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
                                text: "Describe the image".to_owned(),
                            },
                        ]),
                        role: "user".to_owned(),
                    },
                ]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert_eq!(
        collected.into_token_results(),
        vec![GeneratedTokenResult::MultimodalTokenizationFailed(
            "test-agent: failed to tokenize multimodal input: llama.cpp returned 2: media preprocessing failed, or its tokenizer threw (text too large, missing vocab, media marker and bitmap count mismatch, or allocation failure)".to_owned()
        )]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
