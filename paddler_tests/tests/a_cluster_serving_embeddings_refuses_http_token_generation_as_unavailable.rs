use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

const REFUSAL: &str = "The cluster serves Embeddings, not TextGeneration";

fn is_refused_as_unavailable(rejection: &ClientError) -> bool {
    matches!(
        rejection,
        ClientError::ServiceUnavailable { message, .. } if message == REFUSAL
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_serving_embeddings_refuses_http_token_generation_as_unavailable() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Embeddings))
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
        .expect("an embeddings cluster must refuse a raw prompt");
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
        .expect("an embeddings cluster must refuse a conversation history");

    assert!(is_refused_as_unavailable(&raw_prompt_rejection));
    assert!(is_refused_as_unavailable(&conversation_history_rejection));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
