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

/// When a client disconnects (receiver dropped), the scheduler must detect
/// the failed send and release the slot. A subsequent request on the same
/// slot must eventually succeed.
#[actix_web::test]
async fn test_generation_stops_when_client_disconnects() -> Result<()> {
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

    // Start generation, receive one token, then disconnect
    let (gen_tx, mut gen_rx) = mpsc::unbounded_channel();
    let (_, gen_stop_rx) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx: gen_tx,
                generate_tokens_stop_rx: gen_stop_rx,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 500,
                    raw_prompt: "Write a very long essay about philosophy".to_string(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send first command: {err}"))?;

    // Wait for generation to start
    let first = gen_rx.recv().await;
    assert!(matches!(first, Some(GeneratedTokenResult::Token(_))));

    // Client disconnects
    drop(gen_rx);

    // Send requests until the scheduler releases the slot from the
    // disconnected client. Without the fix, the slot is never released
    // and every attempt gets SamplerError.
    let max_attempts = 10;
    let mut succeeded = false;

    for attempt in 0..max_attempts {
        let (gen_tx_retry, gen_rx_retry) = mpsc::unbounded_channel();
        let (_, gen_stop_rx_retry) = mpsc::unbounded_channel::<()>();

        managed_model
            .handle()
            .command_tx
            .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
                ContinueFromRawPromptRequest {
                    generated_tokens_tx: gen_tx_retry,
                    generate_tokens_stop_rx: gen_stop_rx_retry,
                    params: ContinueFromRawPromptParams {
                        grammar: None,
                        max_tokens: 5,
                        raw_prompt: "Hello".to_string(),
                    },
                },
            ))
            .map_err(|err| anyhow::anyhow!("Failed to send retry command: {err}"))?;

        let results = collect_generated_tokens(gen_rx_retry).await?;

        let has_tokens = results
            .iter()
            .any(|result| matches!(result, GeneratedTokenResult::Token(_)));

        if has_tokens {
            eprintln!("Slot released after {attempt} retries");
            succeeded = true;

            break;
        }
    }

    assert!(
        succeeded,
        "Slot must be released within {max_attempts} attempts after client disconnect"
    );

    managed_model.shutdown()?;

    Ok(())
}
