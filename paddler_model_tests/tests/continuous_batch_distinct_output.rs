#![cfg(feature = "tests_that_use_llms")]

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

#[actix_web::test]
async fn test_continuous_batch_produces_distinct_output_per_sequence() -> Result<()> {
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

    let prompt_a = "Count from one to ten in English: one, two,";
    let prompt_b = "The capital of France is";

    let (tx_a, rx_a) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx_a) = mpsc::unbounded_channel::<()>();
    let (tx_b, rx_b) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx_b) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx: tx_a,
                generate_tokens_stop_rx: stop_rx_a,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 20,
                    raw_prompt: prompt_a.to_string(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx: tx_b,
                generate_tokens_stop_rx: stop_rx_b,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 20,
                    raw_prompt: prompt_b.to_string(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

    let (results_a, results_b) = tokio::join!(
        collect_generated_tokens(rx_a),
        collect_generated_tokens(rx_b),
    );

    let results_a = results_a?;
    let results_b = results_b?;

    let text_a: String = results_a
        .iter()
        .filter_map(|result| match result {
            GeneratedTokenResult::Token(token) => Some(token.as_str()),
            _ => None,
        })
        .collect();

    let text_b: String = results_b
        .iter()
        .filter_map(|result| match result {
            GeneratedTokenResult::Token(token) => Some(token.as_str()),
            _ => None,
        })
        .collect();

    eprintln!("Output A (counting): {text_a}");
    eprintln!("Output B (capital): {text_b}");

    assert_ne!(
        text_a, text_b,
        "Two different prompts should produce different outputs"
    );

    let token_count_a = results_a
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();
    let token_count_b = results_b
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(token_count_a > 0, "Prompt A should produce tokens");
    assert!(token_count_b > 0, "Prompt B should produce tokens");

    managed_model.shutdown()?;

    Ok(())
}
