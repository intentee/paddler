#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use paddler_agent::continuous_batch_embedding_processor::ContinuousBatchEmbeddingProcessor;
use paddler_agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use paddler_agent::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent::slot_guard::SlotGuard;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::pooling_type::PoolingType;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_tests::embedding_processor_harness::EmbeddingProcessorHarness;
use tokio::sync::mpsc;

#[test]
fn agent_embedding_processor_produces_embedding_for_valid_document() -> Result<()> {
    let harness = EmbeddingProcessorHarness::build_for_embedding_generation(InferenceParameters {
        enable_embeddings: true,
        ..InferenceParameters::default()
    })?;
    let expected_embedding_dimension = usize::try_from(harness.scheduler_context.model.n_embd())?;

    let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();
    let (_generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();
    let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(1));

    let request = GenerateEmbeddingBatchRequest {
        generate_embedding_stop_rx,
        generated_embedding_tx,
        params: GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "hello".to_owned(),
                id: "doc-1".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        },
        slot_guard: SlotGuard::new(slot_aggregated_status),
    };

    let mut processor =
        ContinuousBatchEmbeddingProcessor::new(harness.llama_context, &harness.scheduler_context);

    let result = processor.process_embedding_batch(request);

    assert!(result.is_ok());

    let EmbeddingResult::Embedding(embedding) = generated_embedding_rx.try_recv()? else {
        return Err(anyhow!(
            "expected an embedding result for the valid document"
        ));
    };

    assert_eq!(embedding.source_document_id, "doc-1");
    assert!(matches!(
        embedding.normalization_method,
        EmbeddingNormalizationMethod::None
    ));
    assert_eq!(embedding.pooling_type, PoolingType::Last);
    assert_eq!(embedding.embedding.len(), expected_embedding_dimension);

    assert!(matches!(
        generated_embedding_rx.try_recv()?,
        EmbeddingResult::Done
    ));

    Ok(())
}
