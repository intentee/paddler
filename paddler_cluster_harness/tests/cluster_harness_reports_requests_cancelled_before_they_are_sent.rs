use std::mem::discriminant;
use std::num::NonZeroU32;

use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

#[tokio::test(flavor = "multi_thread")]
async fn cluster_harness_reports_requests_cancelled_before_they_are_sent() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");
    let cancelled_requests = CancellationToken::new();

    cancelled_requests.cancel();

    let request_errors = [
        cluster
            .continue_from_raw_prompt(
                cancelled_requests.clone(),
                &ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::MIN,
                    raw_prompt: "Paddler".to_owned(),
                },
            )
            .await
            .err(),
        cluster
            .continue_from_conversation_history(
                cancelled_requests.clone(),
                &ContinueFromConversationHistoryParams {
                    add_generation_prompt: true,
                    conversation_history: ConversationHistory::new(Vec::new()),
                    enable_thinking: ModelDefaultOr::Explicit(false),
                    grammar: None,
                    max_tokens: NonZeroU32::MIN,
                    parallel_tool_calls: ModelDefaultOr::ModelDefault,
                    parse_tool_calls: false,
                    tools: Vec::new(),
                },
            )
            .await
            .err(),
        cluster
            .decide(
                cancelled_requests.clone(),
                &DecisionRequest {
                    images: Vec::new(),
                    questions: IndexMap::new(),
                    state: json!("Paddler"),
                },
            )
            .await
            .err(),
        cluster
            .generate_embedding_batch(
                cancelled_requests,
                &GenerateEmbeddingBatchParams {
                    input_batch: Vec::new(),
                    normalization_method: EmbeddingNormalizationMethod::None,
                },
            )
            .await
            .err(),
    ];

    assert_eq!(
        request_errors.map(|request_error| request_error
            .as_ref()
            .and_then(|request_error| request_error.root_cause().downcast_ref::<ClientError>())
            .map(discriminant)),
        [Some(discriminant(&ClientError::RequestCancelled {
            url: String::new()
        })); 4]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
