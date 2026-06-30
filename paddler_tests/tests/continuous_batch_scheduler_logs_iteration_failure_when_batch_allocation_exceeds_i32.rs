#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;
use std::sync::mpsc::channel;

use anyhow::Result;
use log::LevelFilter;
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
fn continuous_batch_scheduler_logs_iteration_failure_when_batch_allocation_exceeds_i32()
-> Result<()> {
    log::set_max_level(LevelFilter::Trace);

    let loaded = LoadedTestModel::qwen3()?;
    let llama_context = loaded.new_context()?;
    let scheduler_context = loaded.scheduler_context(InferenceParameters {
        n_batch: usize::MAX,
        ..InferenceParameters::default()
    })?;
    let (command_tx, command_rx) = channel();
    let mut scheduler =
        ContinuousBatchScheduler::new(command_rx, scheduler_context, llama_context, 1);

    let (generated_tokens_tx, _generated_tokens_rx) = mpsc::unbounded_channel();
    let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();
    let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(1));
    let request = ContinueFromRawPromptRequest {
        generate_tokens_stop_rx,
        generated_tokens_tx,
        params: ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 8,
            raw_prompt: "Hi".to_owned(),
        },
        slot_guard: SlotGuard::new(slot_aggregated_status),
    };

    command_tx.send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
        request,
    ))?;
    drop(command_tx);

    scheduler.run();

    Ok(())
}
