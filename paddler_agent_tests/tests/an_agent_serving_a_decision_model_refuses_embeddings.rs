#![cfg(feature = "tests_that_use_llms")]

use tokio::sync::mpsc;

use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_serving_a_decision_model_refuses_embeddings() {
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(DECISION_SLOTS_MINIMUM.get())
        .expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(DecisionDesiredState::default().into_balancer_desired_state())
        .await
        .expect("the desired state must be served");

    let (response_tx, mut response_rx) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx) = mpsc::unbounded_channel();

    serving_pipeline_arbiter
        .pipeline_arbiter
        .forward(PipelineRequest::GenerateEmbeddingBatch(AgentRequest {
            params: GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "the quick brown fox jumps over the lazy dog".to_owned(),
                    id: "doc-1".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
            response_tx,
            slot_guard: serving_pipeline_arbiter.slot_guard(DECISION_SLOTS_MINIMUM),
            stop_rx,
        }));

    assert!(matches!(
        response_rx.recv().await,
        Some(EmbeddingResult::EmbeddingsNotServed(_))
    ));

    serving_pipeline_arbiter.shut_down().await;
}
