#![cfg(all(feature = "tests_that_use_llms", feature = "cuda"))]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_model_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model_params::ManagedModelParams;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use tokio::sync::mpsc;

mod cuda_common;

use cuda_common::require_cuda_device;

const QWEN3_0_6B_LAYER_COUNT: u32 = 28;

#[actix_web::test]
async fn cuda_continuous_batch_smoke_uses_gpu() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    require_cuda_device()?;

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters {
            n_gpu_layers: QWEN3_0_6B_LAYER_COUNT,
            ..InferenceParameters::default()
        },
        model: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
        multimodal_projection: None,
        slots: 1,
    })
    .await?;

    let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();
    let (_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx,
                generate_tokens_stop_rx,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 16,
                    raw_prompt: "Count from 1 to 5:".to_owned(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

    let results = collect_generated_tokens(generated_tokens_rx).await?;

    let token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(token_count > 0, "CUDA smoke test produced no tokens");
    assert!(matches!(results.last(), Some(GeneratedTokenResult::Done)));

    managed_model.shutdown()?;

    Ok(())
}
