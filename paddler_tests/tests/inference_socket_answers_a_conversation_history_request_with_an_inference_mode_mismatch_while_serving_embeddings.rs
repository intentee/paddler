use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_messaging::inference_client::response::Response as InferenceClientResponse;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_answers_a_conversation_history_request_with_an_inference_mode_mismatch_while_serving_embeddings()
 {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Embeddings))
        .await
        .expect("a balancer serving embeddings must start");

    let answer = cluster
        .client_inference
        .continue_from_conversation_history(
            CancellationToken::new(),
            ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("What is the weather?".to_owned()),
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
        .expect("the inference socket must accept a conversation history request")
        .next()
        .await
        .expect("the inference socket must answer the conversation history request")
        .expect("the answer must be readable");

    assert!(matches!(
        answer,
        InferenceClientMessage::Response(envelope)
            if matches!(
                &envelope.response,
                InferenceClientResponse::GeneratedToken(
                    GeneratedTokenResult::InferenceModeMismatch(description)
                ) if description == "The cluster serves Embeddings, not TextGeneration"
            )
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
