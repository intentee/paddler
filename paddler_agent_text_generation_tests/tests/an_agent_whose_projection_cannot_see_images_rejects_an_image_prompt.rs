#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_balancer_desired_state_fixture::ultravox_desired_state::ultravox_desired_state;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_file_fixture::image_fixture::ImageFixture;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_whose_projection_cannot_see_images_rejects_an_image_prompt() {
    let cluster = Cluster::start_in_process(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: ClusterDesiredState::Apply(Box::new(ultravox_desired_state())),
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
                                    url: ImageFixture::LlamasJpeg
                                        .data_uri()
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
                enable_thinking: ModelDefaultOr::Explicit(false),
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parallel_tool_calls: ModelDefaultOr::ModelDefault,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert!(matches!(
        collected.into_token_results().as_slice(),
        [GeneratedTokenResult::MultimodalNotSupported(_)]
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
