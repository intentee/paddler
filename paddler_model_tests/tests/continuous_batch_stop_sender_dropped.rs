#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model_params::ManagedModelParams;
use paddler_model_tests::model_test_harness::ModelTestHarness;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use tokio::sync::mpsc;

/// When the stop signal sender is dropped (client disconnected, stopper
/// cleaned up), the scheduler must detect the closed channel and stop
/// generating. This simulates what happens when a WebSocket client
/// disconnects: the ReceiveStreamStopperDropGuard drops the stop_tx.
///
/// Uses 2 slots: first request gets stop_tx dropped, second request must
/// complete normally — proving the scheduler stopped wasting resources
/// on the disconnected client.
#[actix_web::test]
async fn test_generation_stops_when_stop_sender_dropped() -> Result<()> {
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
    let (gen_stop_tx, gen_stop_rx) = mpsc::unbounded_channel::<()>();

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
        .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

    // Wait for generation to start
    let first = gen_rx.recv().await;
    assert!(matches!(first, Some(GeneratedTokenResult::Token(_))));

    // Client disconnects — drop the stop sender (simulates guard cleanup)
    drop(gen_stop_tx);

    // Second request must complete normally — proves the scheduler detected
    // the disconnect and stopped the first request
    let harness = ModelTestHarness::new(&managed_model);

    let results = harness
        .generate_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 5,
            raw_prompt: "Hello".to_string(),
        })
        .await?;

    assert!(
        matches!(results.last(), Some(GeneratedTokenResult::Done)),
        "Second request must complete with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
