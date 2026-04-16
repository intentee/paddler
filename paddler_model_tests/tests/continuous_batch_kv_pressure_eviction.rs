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

/// Two concurrent generations with combined KV footprint larger than the
/// allocated context force the scheduler down the `DecodeError::NoKvCacheSlot`
/// path. Previously, the scheduler recursed into `execute_one_iteration`
/// unboundedly on each eviction attempt, risking a stack overflow if the
/// remaining sequence still couldn't fit. The scheduler must now terminate
/// deterministically: at least one request completes with `Done`, and the
/// test returns within the timeout rather than hanging or crashing.
#[actix_web::test]
async fn test_eviction_terminates_and_survivor_completes() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    // Per llama.cpp's non-unified KV cache layout, `n_ctx_seq = n_ctx / n_seq_max`:
    // with context_size=256 and 2 slots, each sequence has a 128-token KV budget.
    // A long prompt plus enough max_tokens pushes the larger sequence past that
    // budget and forces `NoKvCacheSlot` during decode. The short sequence stays
    // within budget and completes after the large one is evicted.
    //
    // Sampling is pinned to greedy (temperature=0) so the long sequence's decoded
    // continuation is deterministic across runs — whether it hits EOS before
    // exceeding the per-sequence KV budget is then a property of the model, not
    // of the sampler's random draws.
    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters {
            batch_n_tokens: 256,
            context_size: 256,
            temperature: 0.0,
            ..InferenceParameters::default()
        },
        model: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
        multimodal_projection: None,
        slots: 2,
    })
    .await?;

    let long_prompt = "Describe in great detail how the process of photosynthesis \
        works in plants. Cover the light-dependent reactions, the Calvin cycle, \
        the role of chlorophyll, the thylakoid membrane, and the stroma. \
        Explain how water and carbon dioxide are converted to glucose and oxygen. \
        Discuss the evolutionary history of this process and its importance \
        throughout the biosphere, and then give a long essay response.";
    let short_prompt = "Hi";

    let (tx_a, rx_a) = mpsc::unbounded_channel();
    let (_stop_tx_a, stop_rx_a) = mpsc::unbounded_channel::<()>();
    let (tx_b, rx_b) = mpsc::unbounded_channel();
    let (_stop_tx_b, stop_rx_b) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx: tx_a,
                generate_tokens_stop_rx: stop_rx_a,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 200,
                    raw_prompt: long_prompt.to_owned(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send first command: {err}"))?;

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
                    raw_prompt: short_prompt.to_owned(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send second command: {err}"))?;

    let (results_a, results_b) = tokio::join!(
        collect_generated_tokens(rx_a),
        collect_generated_tokens(rx_b)
    );

    let results_a = results_a?;
    let results_b = results_b?;

    let long_was_evicted = results_a.iter().any(|result| {
        matches!(result, GeneratedTokenResult::SamplerError(message) if message.contains("evicted"))
    });
    let short_completed_with_done = matches!(results_b.last(), Some(GeneratedTokenResult::Done));

    assert!(
        long_was_evicted,
        "The long prompt must be evicted (it exhausts its per-sequence KV budget \
         first); got results_a={results_a:?}"
    );
    assert!(
        short_completed_with_done,
        "The short prompt must complete with Done after the long one is evicted; \
         got results_b={results_b:?}"
    );

    managed_model.shutdown()?;

    Ok(())
}
