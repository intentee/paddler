#![cfg(feature = "tests_that_use_llms")]

use tokio::sync::mpsc;

use paddler_agent::desired_state_reconciliation::DesiredStateReconciliation;
use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent_decision::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_test_cluster_harness::pointer_head_fixture::pointer_head_fixture;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE;
use paddler_tests::serving_pipeline_arbiter::ServingPipelineArbiter;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_serving_decisions_rejects_embedding_batches() {
    let serving_pipeline_arbiter = ServingPipelineArbiter::start(
        qwen3_5_0_8b().into_decision_desired_state(pointer_head_fixture(
            QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE,
        )),
        DECISION_SLOTS_MINIMUM,
    )
    .await;

    assert_eq!(
        serving_pipeline_arbiter.desired_state_reconciliation,
        DesiredStateReconciliation::Reconciled
    );

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

    serving_pipeline_arbiter
        .shut_down()
        .await
        .expect("the agent must shut down cleanly");
}
