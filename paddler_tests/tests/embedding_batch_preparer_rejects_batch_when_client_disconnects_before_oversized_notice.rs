#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;
use std::sync::Arc;

use anyhow::Result;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use paddler_agent::embedding_batch_preparer::EmbeddingBatchPreparer;
use paddler_agent::embedding_batch_rejection::EmbeddingBatchRejection;
use paddler_agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_tests::detached_slot_guard::detached_slot_guard;
use paddler_tests::load_model_from_card::load_model_from_card;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

const SINGLE_TOKEN_BATCH: usize = 1;

#[test]
fn embedding_batch_preparer_rejects_batch_when_client_disconnects_before_oversized_notice()
-> Result<()> {
    let llama_backend = LlamaBackend::init()?;
    let preparer = EmbeddingBatchPreparer {
        enable_embeddings: true,
        model: Arc::new(load_model_from_card(&llama_backend, qwen3_0_6b())?),
        n_batch: SINGLE_TOKEN_BATCH,
    };
    let (generated_embedding_tx, generated_embedding_rx) = mpsc::unbounded_channel();
    let (_generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

    drop(generated_embedding_rx);

    let rejection = preparer
        .prepare(
            None,
            GenerateEmbeddingBatchRequest {
                generate_embedding_stop_rx,
                generated_embedding_tx,
                params: GenerateEmbeddingBatchParams {
                    input_batch: vec![EmbeddingInputDocument {
                        content: "A document longer than a single token".to_owned(),
                        id: "oversized".to_owned(),
                    }],
                    normalization_method: EmbeddingNormalizationMethod::None,
                },
                slot_guard: detached_slot_guard(),
            },
        )
        .err()
        .map(|rejection| discriminant(&rejection));

    assert_eq!(
        rejection,
        Some(discriminant(&EmbeddingBatchRejection::ClientDisconnected(
            SendError(EmbeddingResult::Done)
        )))
    );

    Ok(())
}
