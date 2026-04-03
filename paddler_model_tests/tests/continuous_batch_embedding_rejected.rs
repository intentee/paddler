#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model_params::ManagedModelParams;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::embedding_result::EmbeddingResult;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use tokio::sync::mpsc;

#[actix_web::test]
async fn test_embedding_rejected_during_active_generation() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters::default(),
        model: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_string(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_string(),
            revision: "main".to_string(),
        },
        multimodal_projection: None,
        slots: 2,
    })
    .await?;

    let (gen_tx, mut gen_rx) = mpsc::unbounded_channel();
    let (_stop_tx, gen_stop_rx) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx: gen_tx,
                generate_tokens_stop_rx: gen_stop_rx,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 50,
                    raw_prompt: "Tell me a long story about a cat".to_string(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send generation command: {err}"))?;

    gen_rx.recv().await;

    let (embed_tx, mut embed_rx) = mpsc::unbounded_channel();
    let (_stop_tx, embed_stop_rx) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::GenerateEmbeddingBatch(
            GenerateEmbeddingBatchRequest {
                generated_embedding_tx: embed_tx,
                generate_embedding_stop_rx: embed_stop_rx,
                params: GenerateEmbeddingBatchParams {
                    input_batch: vec![EmbeddingInputDocument {
                        content: "test".to_string(),
                        id: "doc1".to_string(),
                    }],
                    normalization_method: EmbeddingNormalizationMethod::None,
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send embedding command: {err}"))?;

    let embedding_result = embed_rx.recv().await;

    assert!(
        matches!(embedding_result, Some(EmbeddingResult::Error(_))),
        "Embedding request should be rejected with error during active generation, got: {embedding_result:?}"
    );

    while let Some(token) = gen_rx.recv().await {
        if matches!(token, GeneratedTokenResult::Done) {
            break;
        }
    }

    managed_model.shutdown()?;

    Ok(())
}
