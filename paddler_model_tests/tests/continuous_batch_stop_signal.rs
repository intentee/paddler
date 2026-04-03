#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model_params::ManagedModelParams;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use tokio::sync::mpsc;

#[actix_web::test]
async fn test_stop_signal_terminates_generation_early() -> Result<()> {
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

    let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
    let (generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx,
                generate_tokens_stop_rx,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 500,
                    raw_prompt: "Write a very long story about a dragon".to_string(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

    // Wait for first token to confirm generation started
    let first = generated_tokens_rx.recv().await;
    assert!(
        matches!(first, Some(GeneratedTokenResult::Token(_))),
        "Expected first result to be a token, got: {first:?}"
    );

    // Send stop signal
    generate_tokens_stop_tx.send(())?;

    // Collect remaining tokens
    let mut token_count = 1; // counting the first token
    while let Some(result) = generated_tokens_rx.recv().await {
        match result {
            GeneratedTokenResult::Token(_) => token_count += 1,
            GeneratedTokenResult::Done => break,
            _ => break,
        }
    }

    assert!(
        token_count < 500,
        "Stop signal should terminate generation early, got {token_count} tokens"
    );

    managed_model.shutdown()?;

    Ok(())
}
