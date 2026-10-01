use std::num::NonZeroU32;

use http::StatusCode;
use tokio_util::sync::CancellationToken;

use paddler_balancer::cluster_token_generation_mode::TOKEN_GENERATION_DISABLED_MESSAGE;
use paddler_client::error::Error as ClientError;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

fn is_token_generation_disabled_rejection(rejection: &ClientError) -> bool {
    matches!(
        rejection,
        ClientError::UnexpectedResponseStatus { message, status, .. }
            if *status == StatusCode::NOT_IMPLEMENTED
                && message == TOKEN_GENERATION_DISABLED_MESSAGE
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn balancer_rejects_http_token_generation_with_501_while_the_cluster_serves_embeddings() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        desired_state: Some(BalancerDesiredState {
            inference_parameters: InferenceParameters {
                enable_embeddings: true,
                ..InferenceParameters::default()
            },
            ..BalancerDesiredState::default()
        }),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer serving embeddings must start");

    let raw_prompt_rejection = cluster
        .client_inference
        .post_continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "hello".to_owned(),
            },
        )
        .await
        .err()
        .expect("the raw prompt route must reject token generation in embeddings mode");
    let conversation_history_rejection = cluster
        .client_inference
        .post_continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("hello".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: false,
                tools: Vec::new(),
            },
        )
        .await
        .err()
        .expect("the conversation history route must reject token generation in embeddings mode");

    assert!(is_token_generation_disabled_rejection(
        &raw_prompt_rejection
    ));
    assert!(is_token_generation_disabled_rejection(
        &conversation_history_rejection
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
