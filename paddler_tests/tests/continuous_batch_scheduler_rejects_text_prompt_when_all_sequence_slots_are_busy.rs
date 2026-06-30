#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;
use std::sync::mpsc::channel;

use anyhow::Result;
use anyhow::anyhow;
use paddler_agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler_agent::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_agent::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent::slot_guard::SlotGuard;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::loaded_test_model::LoadedTestModel;
use tokio::sync::mpsc;

fn raw_prompt_command(
    generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
) -> ContinuousBatchSchedulerCommand {
    ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(ContinueFromRawPromptRequest {
        generate_tokens_stop_rx,
        generated_tokens_tx,
        params: ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 1,
            raw_prompt: "Hello".to_owned(),
        },
        slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
    })
}

#[test]
fn continuous_batch_scheduler_rejects_text_prompt_when_all_sequence_slots_are_busy() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let scheduler_context = loaded.scheduler_context(InferenceParameters::default())?;
    let llama_context = loaded.new_context()?;

    let (command_tx, command_rx) = channel::<ContinuousBatchSchedulerCommand>();
    let mut scheduler =
        ContinuousBatchScheduler::new(command_rx, scheduler_context, llama_context, 1);

    let (occupying_tokens_tx, _occupying_tokens_rx) = mpsc::unbounded_channel();
    let (_occupying_stop_tx, occupying_stop_rx) = mpsc::unbounded_channel();
    command_tx
        .send(raw_prompt_command(occupying_tokens_tx, occupying_stop_rx))
        .map_err(|send_error| anyhow!("failed to queue occupying prompt: {send_error}"))?;

    let (rejected_tokens_tx, mut rejected_tokens_rx) = mpsc::unbounded_channel();
    let (_rejected_stop_tx, rejected_stop_rx) = mpsc::unbounded_channel();
    command_tx
        .send(raw_prompt_command(rejected_tokens_tx, rejected_stop_rx))
        .map_err(|send_error| anyhow!("failed to queue rejected prompt: {send_error}"))?;

    drop(command_tx);

    scheduler.run();

    assert!(matches!(
        rejected_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::SamplerError(_))
    ));

    Ok(())
}
