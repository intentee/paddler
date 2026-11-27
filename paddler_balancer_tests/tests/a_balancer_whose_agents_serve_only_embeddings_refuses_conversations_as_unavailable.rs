#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_whose_agents_serve_only_embeddings_refuses_conversations_as_unavailable() {
    let cluster =
        Cluster::start_in_process(EmbeddingClusterParams::default().into_cluster_params())
            .await
            .expect("a cluster serving an embedding model must start");

    let refusal = cluster
        .client_inference
        .post_continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![
                    ConversationMessage::Message {
                        content: ConversationMessageContent::Text("Hello".to_owned()),
                        role: "user".to_owned(),
                    },
                ]),
                enable_thinking: ModelDefaultOr::ModelDefault,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parallel_tool_calls: ModelDefaultOr::ModelDefault,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .err()
        .expect("a cluster serving only embeddings must refuse conversations");

    assert!(matches!(refusal, ClientError::ServiceUnavailable { .. }));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
