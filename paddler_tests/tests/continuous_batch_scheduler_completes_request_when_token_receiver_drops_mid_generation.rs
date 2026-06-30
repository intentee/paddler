#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;

use anyhow::Result;
use anyhow::anyhow;
use paddler_agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler_agent::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_agent::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent::slot_guard::SlotGuard;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::loaded_test_model::LoadedTestModel;
use tokio::sync::mpsc;

#[test]
fn continuous_batch_scheduler_completes_request_when_token_receiver_drops_mid_generation()
-> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let scheduler_context = loaded.scheduler_context(InferenceParameters::default())?;
    let llama_context = loaded.new_context()?;

    let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(1));

    let (command_tx, command_rx) = channel::<ContinuousBatchSchedulerCommand>();
    let mut scheduler =
        ContinuousBatchScheduler::new(command_rx, scheduler_context, llama_context, 1);

    let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();
    let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

    drop(generated_tokens_rx);

    command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generate_tokens_stop_rx,
                generated_tokens_tx,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 32,
                    raw_prompt: "Count to three:".to_owned(),
                },
                slot_guard: SlotGuard::new(Arc::clone(&slot_aggregated_status)),
            },
        ))
        .map_err(|send_error| anyhow!("failed to queue raw prompt command: {send_error}"))?;

    let slot_aggregated_status_for_controller = Arc::clone(&slot_aggregated_status);
    let controller = thread::spawn(move || {
        while slot_aggregated_status_for_controller.slots_processing_count() > 0 {
            thread::yield_now();
        }

        drop(command_tx);
    });

    scheduler.run();

    controller
        .join()
        .map_err(|_| anyhow!("controller thread panicked while waiting for slot release"))?;

    assert_eq!(slot_aggregated_status.slots_processing_count(), 0);

    Ok(())
}
