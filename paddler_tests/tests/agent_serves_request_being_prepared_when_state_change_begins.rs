#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio::spawn;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::load_fixture_data_uri::load_fixture_data_uri;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::desired_state_with_halved_image_resize::desired_state_with_halved_image_resize;
use paddler_tests::smolvlm2_desired_state::smolvlm2_desired_state;
use paddler_tests::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;

const SLOW_TO_DECODE_IMAGE_FIXTURE: &str = "solid_color_4000x4000.png";

#[tokio::test(flavor = "multi_thread")]
async fn agent_serves_request_being_prepared_when_state_change_begins() {
    let mut cluster = start_cluster_with_smolvlm2(AgentConfig::uniform(1, 2))
        .await
        .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();

    let request_being_prepared = spawn(cluster.continue_from_conversation_history(
        CancellationToken::new(),
        &ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Parts(vec![
                        ConversationMessageContentPart::ImageUrl {
                            image_url: ImageUrl {
                                url: load_fixture_data_uri(
                                    SLOW_TO_DECODE_IMAGE_FIXTURE,
                                    "image/png",
                                )
                                .expect("the test fixture must load"),
                            },
                        },
                        ConversationMessageContentPart::Text {
                            text: "What color is this image?".to_owned(),
                        },
                    ]),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: NonZeroU32::new(4).unwrap(),
            parse_tool_calls: false,
            tools: vec![],
        },
    ));

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .expect("the image request must be dispatched to the agent first");

    let later_request_cancellation = CancellationToken::new();
    let mut later_request_stream = cluster
        .continue_from_raw_prompt_stream(later_request_cancellation.clone(), &unending_generation())
        .await
        .expect("the inference request must be accepted");

    later_request_stream
        .next()
        .await
        .expect("the later request must stream a token after the image request started preparing")
        .expect("the message must be readable");

    let initial_desired_state = smolvlm2_desired_state();

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &desired_state_with_halved_image_resize(initial_desired_state)
                .expect("the desired state must be derivable"),
        )
        .await
        .expect("the balancer must accept the desired state");

    cluster
        .agents_watcher
        .until_agent(&agent_id, |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.status.state_application_status != AgentStateApplicationStatus::Applied
            })
        })
        .await
        .expect("the agent must start applying the changed state");

    later_request_cancellation.cancel();
    drop(later_request_stream);

    let collected = request_being_prepared
        .await
        .expect("the image request task must not panic")
        .expect("the message must be readable");

    assert!(
        matches!(
            collected
                .token_results
                .last()
                .map(|token_result_with_producer| &token_result_with_producer.token_result),
            Some(GeneratedTokenResult::Done(_))
        ),
        "the request that was being prepared when the state change began must be served to completion"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
