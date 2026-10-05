use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

const BAD_REQUEST: u16 = 400;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_rejects_tool_call_parsing_without_tools() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let rejection = cluster
        .client_inference
        .post_continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("What is the weather?".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: true,
                tools: Vec::new(),
            },
        )
        .await
        .err();

    assert!(matches!(
        rejection,
        Some(ClientError::UnexpectedResponseStatus { message, status, .. })
            if status.as_u16() == BAD_REQUEST
                && message == "Invalid request parameters: parse_tool_calls requires at least one tool"
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
