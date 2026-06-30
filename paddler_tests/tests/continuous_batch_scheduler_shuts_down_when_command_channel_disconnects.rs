#![cfg(feature = "tests_that_use_llms")]

use std::sync::mpsc::channel;

use anyhow::Result;
use log::LevelFilter;
use paddler_agent::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_tests::loaded_test_model::LoadedTestModel;

#[test]
fn continuous_batch_scheduler_shuts_down_when_command_channel_disconnects() -> Result<()> {
    log::set_max_level(LevelFilter::Trace);

    let loaded = LoadedTestModel::qwen3()?;
    let scheduler_context = loaded.scheduler_context(InferenceParameters::default())?;
    let llama_context = loaded.new_context()?;

    let (command_tx, command_rx) = channel::<ContinuousBatchSchedulerCommand>();

    let mut scheduler =
        ContinuousBatchScheduler::new(command_rx, scheduler_context, llama_context, 1);

    drop(command_tx);

    scheduler.run();

    Ok(())
}
