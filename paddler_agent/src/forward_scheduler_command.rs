use std::sync::mpsc::SendError;
use std::sync::mpsc::Sender;

use log::info;

use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::embedding_batch_rejection::EmbeddingBatchRejection;
use crate::generation_request_rejection::GenerationRequestRejection;

pub fn forward_scheduler_command(
    scheduler_command_tx: &Sender<ContinuousBatchSchedulerCommand>,
    agent_name: Option<&str>,
    command: ContinuousBatchSchedulerCommand,
) {
    if let Err(SendError(unsent_command)) = scheduler_command_tx.send(command) {
        match unsent_command {
            ContinuousBatchSchedulerCommand::Generate(request) => {
                GenerationRequestRejection::SchedulerUnavailable
                    .report(agent_name, &request.generated_tokens_tx);
            }
            ContinuousBatchSchedulerCommand::GenerateEmbeddingBatch(request) => {
                EmbeddingBatchRejection::SchedulerUnavailable
                    .report(agent_name, &request.generated_embedding_tx);
            }
            ContinuousBatchSchedulerCommand::Shutdown => {
                info!("{agent_name:?}: scheduler stopped before the shutdown command arrived");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::sync::Arc;
    use std::sync::mpsc::channel;

    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use tokio::sync::mpsc;

    use super::forward_scheduler_command;
    use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
    use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
    use crate::prepared_generation_request::PreparedGenerationRequest;
    use crate::prepared_prompt::PreparedPrompt;
    use crate::slot_aggregated_status::SlotAggregatedStatus;
    use crate::slot_guard::SlotGuard;

    fn slot_guard() -> SlotGuard {
        SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1)))
    }

    #[test]
    fn delivers_command_to_a_running_scheduler() {
        let (scheduler_command_tx, scheduler_command_rx) = channel();

        forward_scheduler_command(
            &scheduler_command_tx,
            None,
            ContinuousBatchSchedulerCommand::Shutdown,
        );

        assert_eq!(
            scheduler_command_rx
                .try_recv()
                .map(|command| discriminant(&command)),
            Ok(discriminant(&ContinuousBatchSchedulerCommand::Shutdown))
        );
    }

    #[test]
    fn rejects_generation_when_the_scheduler_stopped() {
        let (scheduler_command_tx, scheduler_command_rx) = channel();
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

        drop(scheduler_command_rx);

        forward_scheduler_command(
            &scheduler_command_tx,
            Some("agent"),
            ContinuousBatchSchedulerCommand::Generate(Box::new(PreparedGenerationRequest {
                generate_tokens_stop_rx,
                generated_tokens_tx,
                grammar_sampler: None,
                max_tokens: 1,
                prompt: PreparedPrompt::TextTokens(Vec::new()),
                slot_guard: slot_guard(),
                tool_call_pipeline: None,
            })),
        );

        assert_eq!(
            generated_tokens_rx.try_recv(),
            Ok(GeneratedTokenResult::SamplerError(
                "Some(\"agent\"): the scheduler is no longer accepting requests".to_owned()
            ))
        );
    }

    #[test]
    fn rejects_embedding_batch_when_the_scheduler_stopped() {
        let (scheduler_command_tx, scheduler_command_rx) = channel();
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();
        let (_generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

        drop(scheduler_command_rx);

        forward_scheduler_command(
            &scheduler_command_tx,
            Some("agent"),
            ContinuousBatchSchedulerCommand::GenerateEmbeddingBatch(
                PreparedEmbeddingBatchRequest {
                    generate_embedding_stop_rx,
                    generated_embedding_tx,
                    inputs: Vec::new(),
                    normalization_method: EmbeddingNormalizationMethod::None,
                    slot_guard: slot_guard(),
                },
            ),
        );

        assert_eq!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::Error(
                "Some(\"agent\"): the scheduler is no longer accepting requests".to_owned()
            ))
        );
    }

    #[test]
    fn accepts_shutdown_after_the_scheduler_stopped() {
        let (scheduler_command_tx, scheduler_command_rx) = channel();

        drop(scheduler_command_rx);

        forward_scheduler_command(
            &scheduler_command_tx,
            None,
            ContinuousBatchSchedulerCommand::Shutdown,
        );

        assert!(
            scheduler_command_tx
                .send(ContinuousBatchSchedulerCommand::Shutdown)
                .is_err()
        );
    }
}
