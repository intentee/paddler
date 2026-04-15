#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_model_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_model_tests::log_generated_response::log_generated_response;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model_params::ManagedModelParams;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use tokio::sync::mpsc;

#[actix_web::test]
async fn test_long_and_short_prompts_complete_concurrently() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters::default(),
        model: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
        multimodal_projection: None,
        slots: 2,
    })
    .await?;

    let long_prompt = "Explain in great detail how photosynthesis works in plants. \
        Cover the light-dependent reactions, the Calvin cycle, and the role of chlorophyll. \
        Discuss how water and carbon dioxide are converted into glucose and oxygen. \
        Include information about the thylakoid membrane and the stroma.";
    let short_prompt = "Hi";

    let (tx_long, rx_long) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx_long) = mpsc::unbounded_channel::<()>();
    let (tx_short, rx_short) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx_short) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx: tx_long,
                generate_tokens_stop_rx: stop_rx_long,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 20,
                    raw_prompt: long_prompt.to_owned(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send long prompt: {err}"))?;

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx: tx_short,
                generate_tokens_stop_rx: stop_rx_short,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 20,
                    raw_prompt: short_prompt.to_owned(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send short prompt: {err}"))?;

    let (results_long, results_short) = tokio::join!(
        collect_generated_tokens(rx_long),
        collect_generated_tokens(rx_short),
    );

    let results_long = results_long?;
    let results_short = results_short?;

    log_generated_response(&results_long);
    log_generated_response(&results_short);

    let long_tokens = results_long
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();
    let short_tokens = results_short
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(long_tokens > 0, "Long prompt should produce tokens");
    assert!(short_tokens > 0, "Short prompt should produce tokens");

    assert!(
        matches!(results_long.last(), Some(GeneratedTokenResult::Done)),
        "Long prompt should end with Done"
    );
    assert!(
        matches!(results_short.last(), Some(GeneratedTokenResult::Done)),
        "Short prompt should end with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
