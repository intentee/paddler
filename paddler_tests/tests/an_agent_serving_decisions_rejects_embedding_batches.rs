#![cfg(feature = "tests_that_use_llms")]

use tokio::sync::mpsc;

use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::serving_decision_agent::ServingDecisionAgent;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_serving_decisions_rejects_embedding_batches() {
    let serving_decision_agent = ServingDecisionAgent::start().await;
    let serving_pipeline_arbiter = &serving_decision_agent.serving_pipeline_arbiter;
    let (response_tx, mut response_rx) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx) = mpsc::unbounded_channel();

    serving_pipeline_arbiter.pipeline_arbiter_state.forward(
        serving_pipeline_arbiter
            .inference_runtime_context
            .agent_name
            .as_deref(),
        PipelineRequest::GenerateEmbeddingBatch(AgentRequest {
            params: GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "Hello world".to_owned(),
                    id: "doc-1".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
            response_tx,
            slot_guard: serving_pipeline_arbiter.slot_guard(),
            stop_rx,
        }),
    );

    assert_eq!(
        response_rx.recv().await,
        Some(EmbeddingResult::InferenceModeMismatch(
            "agent: the agent serves Decision, so it cannot generate embeddings".to_owned()
        ))
    );

    serving_decision_agent.shut_down().await;
}
