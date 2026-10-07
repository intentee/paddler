use std::num::NonZeroU32;

use http::StatusCode;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

fn is_route_not_found(rejection: &ClientError) -> bool {
    matches!(
        rejection,
        ClientError::UnexpectedResponseStatus { status, .. } if *status == StatusCode::NOT_FOUND
    )
}

fn is_socket_upgrade_not_found(rejection: &ClientError) -> bool {
    matches!(
        rejection,
        ClientError::WebSocket(WebSocketError::Http(response))
            if response.status() == StatusCode::NOT_FOUND
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn an_embeddings_cluster_serves_no_token_generation_routes() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState::unconfigured(
            InferenceMode::Embeddings,
        ))),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer serving embeddings must start");
    let conversation_history_params = ContinueFromConversationHistoryParams {
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
    };

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
        .expect("an embeddings cluster must not serve the raw prompt route");
    let conversation_history_rejection = cluster
        .client_inference
        .post_continue_from_conversation_history(
            CancellationToken::new(),
            &conversation_history_params,
        )
        .await
        .err()
        .expect("an embeddings cluster must not serve the conversation history route");
    let inference_socket_rejection = cluster
        .client_inference
        .continue_from_conversation_history(CancellationToken::new(), conversation_history_params)
        .await
        .err()
        .expect("an embeddings cluster must not serve the inference socket");

    assert!(is_route_not_found(&raw_prompt_rejection));
    assert!(is_route_not_found(&conversation_history_rejection));
    assert!(is_socket_upgrade_not_found(&inference_socket_rejection));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
