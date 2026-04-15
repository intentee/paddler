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
async fn test_continuous_batch_processes_four_concurrent_requests() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters::default(),
        model: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
        multimodal_projection: None,
        slots: 4,
    })
    .await?;

    let prompts = [
        "Write the numbers 1 2 3 4 5",
        "Say hello in three languages",
        "Name three colors",
        "List three animals",
    ];

    let mut receivers = Vec::new();
    let mut stop_senders = Vec::new();

    for prompt in &prompts {
        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();
        let (stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel::<()>();

        managed_model
            .handle()
            .command_tx
            .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
                ContinueFromRawPromptRequest {
                    generated_tokens_tx,
                    generate_tokens_stop_rx,
                    params: ContinueFromRawPromptParams {
                        grammar: None,
                        max_tokens: 30,
                        raw_prompt: (*prompt).to_owned(),
                    },
                },
            ))
            .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

        receivers.push(generated_tokens_rx);
        stop_senders.push(stop_tx);
    }

    let (results_0, results_1, results_2, results_3) = tokio::join!(
        collect_generated_tokens(receivers.remove(0)),
        collect_generated_tokens(receivers.remove(0)),
        collect_generated_tokens(receivers.remove(0)),
        collect_generated_tokens(receivers.remove(0)),
    );

    let results_0 = results_0?;
    let results_1 = results_1?;
    let results_2 = results_2?;
    let results_3 = results_3?;

    let all_results = [&results_0, &results_1, &results_2, &results_3];

    for (index, results) in all_results.iter().enumerate() {
        log_generated_response(results);

        let token_count = results
            .iter()
            .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
            .count();

        assert!(
            token_count > 0,
            "Request {index} should produce at least one token"
        );
        assert!(
            matches!(results.last(), Some(GeneratedTokenResult::Done)),
            "Request {index} should end with Done"
        );
    }

    drop(stop_senders);

    managed_model.shutdown()?;

    Ok(())
}
