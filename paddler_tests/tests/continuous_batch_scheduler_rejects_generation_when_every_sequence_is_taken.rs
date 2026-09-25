#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::mpsc::channel;

use anyhow::Result;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::context::params::LlamaContextParams;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::LlamaModel;
use paddler_agent::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_agent::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent::prepared_prompt::PreparedPrompt;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_tests::detached_slot_guard::detached_slot_guard;
use paddler_tests::load_model_from_card::load_model_from_card;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use tokio::sync::mpsc;

const SEQUENCE_COUNT: i32 = 1;

fn generate_command(
    model: &LlamaModel,
    generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
) -> Result<ContinuousBatchSchedulerCommand> {
    let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

    Ok(ContinuousBatchSchedulerCommand::Generate(Box::new(
        PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            grammar_sampler: None,
            max_tokens: 1,
            prompt: PreparedPrompt::TextTokens(model.str_to_token("Hello", AddBos::Always)?),
            slot_guard: detached_slot_guard(),
            streaming_markers: model.streaming_markers()?,
            tool_call_pipeline: None,
        },
    )))
}

#[test]
fn continuous_batch_scheduler_rejects_generation_when_every_sequence_is_taken() -> Result<()> {
    let llama_backend = LlamaBackend::init()?;
    let model = Arc::new(load_model_from_card(&llama_backend, qwen3_0_6b())?);
    let inference_parameters = InferenceParameters::default();
    let llama_context = LlamaContext::from_model(
        &model,
        &llama_backend,
        LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(inference_parameters.context_size))
            .with_n_batch(u32::try_from(inference_parameters.n_batch)?)
            .with_n_seq_max(u32::try_from(SEQUENCE_COUNT)?),
    )?;
    let (scheduler_command_tx, scheduler_command_rx) = channel();
    let (occupying_tokens_tx, _occupying_tokens_rx) = mpsc::unbounded_channel();
    let (rejected_tokens_tx, mut rejected_tokens_rx) = mpsc::unbounded_channel();

    scheduler_command_tx.send(generate_command(&model, occupying_tokens_tx)?)?;
    scheduler_command_tx.send(generate_command(&model, rejected_tokens_tx)?)?;
    scheduler_command_tx.send(ContinuousBatchSchedulerCommand::Shutdown)?;

    ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
        batch: LlamaBatch::new(inference_parameters.n_batch, SEQUENCE_COUNT)?,
        command_rx: scheduler_command_rx,
        llama_context,
        max_concurrent_sequences: SEQUENCE_COUNT,
        scheduler_context: ContinuousBatchSchedulerContext {
            agent_name: None,
            desired_slots_total: SEQUENCE_COUNT,
            inference_parameters,
            model: model.clone(),
        },
    })
    .run();

    assert_eq!(
        rejected_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::SamplerError(
            "None: no available sequence slots, all slots are busy".to_owned()
        ))
    );

    Ok(())
}
