#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;
use std::sync::mpsc::channel;

use anyhow::Result;
use anyhow::anyhow;
use paddler_agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler_agent::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use paddler_agent::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent::slot_guard::SlotGuard;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_tests::loaded_test_model::LoadedTestModel;
use tokio::sync::mpsc;

#[test]
fn continuous_batch_scheduler_rejects_pending_embedding_while_generating() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let scheduler_context = loaded.scheduler_context(InferenceParameters::default())?;
    let llama_context = loaded.new_context()?;

    let (command_tx, command_rx) = channel::<ContinuousBatchSchedulerCommand>();
    let mut scheduler =
        ContinuousBatchScheduler::new(command_rx, scheduler_context, llama_context, 1);

    let (generated_tokens_tx, _generated_tokens_rx) = mpsc::unbounded_channel();
    let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

    command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generate_tokens_stop_rx,
                generated_tokens_tx,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 1,
                    raw_prompt: "Hello".to_owned(),
                },
                slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
            },
        ))
        .map_err(|send_error| anyhow!("failed to queue raw prompt command: {send_error}"))?;

    let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();
    let (_generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

    command_tx
        .send(ContinuousBatchSchedulerCommand::GenerateEmbeddingBatch(
            GenerateEmbeddingBatchRequest {
                generate_embedding_stop_rx,
                generated_embedding_tx,
                params: GenerateEmbeddingBatchParams {
                    input_batch: vec![EmbeddingInputDocument {
                        content: "hello".to_owned(),
                        id: "doc-1".to_owned(),
                    }],
                    normalization_method: EmbeddingNormalizationMethod::None,
                },
                slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
            },
        ))
        .map_err(|send_error| anyhow!("failed to queue embedding command: {send_error}"))?;

    drop(command_tx);

    scheduler.run();

    assert!(matches!(
        generated_embedding_rx.try_recv(),
        Ok(EmbeddingResult::EmbeddingRejectedDueToActiveTokenGeneration)
    ));

    Ok(())
}
