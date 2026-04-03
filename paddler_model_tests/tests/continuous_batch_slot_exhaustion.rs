#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model::ManagedModelParams;
use paddler_model_tests::model_test_harness::collect_generated_tokens;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use tokio::sync::mpsc;

#[actix_web::test]
async fn test_second_request_rejected_when_single_slot_busy() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters::default(),
        model: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_string(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_string(),
            revision: "main".to_string(),
        },
        multimodal_projection: None,
        slots: 1,
    })
    .await?;

    // First request: occupy the only slot
    let (gen_tx_1, mut gen_rx_1) = mpsc::unbounded_channel();
    let (_stop_tx, gen_stop_rx_1) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx: gen_tx_1,
                generate_tokens_stop_rx: gen_stop_rx_1,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 100,
                    raw_prompt: "Tell me a story".to_string(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send first command: {err}"))?;

    // Wait for first token to confirm slot is occupied
    let first = gen_rx_1.recv().await;
    assert!(
        matches!(first, Some(GeneratedTokenResult::Token(_))),
        "First request should produce a token"
    );

    // Second request: should be rejected (no available slots)
    let (gen_tx_2, gen_rx_2) = mpsc::unbounded_channel();
    let (_stop_tx, gen_stop_rx_2) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx: gen_tx_2,
                generate_tokens_stop_rx: gen_stop_rx_2,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 10,
                    raw_prompt: "Hello".to_string(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send second command: {err}"))?;

    let second_results = collect_generated_tokens(gen_rx_2).await?;

    assert!(
        second_results
            .iter()
            .any(|result| matches!(result, GeneratedTokenResult::SamplerError(_))),
        "Second request should receive SamplerError about no available slots, got: {second_results:?}"
    );

    // Drain first request
    while let Some(token) = gen_rx_1.recv().await {
        if matches!(token, GeneratedTokenResult::Done) {
            break;
        }
    }

    managed_model.shutdown()?;

    Ok(())
}
