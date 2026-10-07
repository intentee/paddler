use tokio::sync::mpsc;

use paddler_agent_runtime::scheduler_command::SchedulerCommand;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

use crate::embedding_error::EmbeddingError;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;

pub struct PreparedEmbeddingBatchRequest {
    pub generate_embedding_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_embedding_tx: mpsc::UnboundedSender<EmbeddingResult>,
    pub inputs: Vec<EmbeddingInputTokenized>,
    pub normalization_method: EmbeddingNormalizationMethod,
    pub oversized_documents: Vec<OversizedEmbeddingDocumentDetails>,
    pub slot_guard: SlotGuard,
}

impl SchedulerCommand for PreparedEmbeddingBatchRequest {
    fn reject_because_the_scheduler_stopped(self, agent_name: Option<&str>) {
        EmbeddingError::SchedulerUnavailable.report(agent_name, &self.generated_embedding_tx);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::mpsc;

    use paddler_agent_runtime::scheduler_command::SchedulerCommand as _;
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_status::slot_guard::SlotGuard;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::embedding_result::EmbeddingResult;

    use super::PreparedEmbeddingBatchRequest;

    #[test]
    fn rejects_an_embedding_batch_when_the_scheduler_stopped() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();
        let (_generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

        PreparedEmbeddingBatchRequest {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            inputs: Vec::new(),
            normalization_method: EmbeddingNormalizationMethod::None,
            oversized_documents: Vec::new(),
            slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
        }
        .reject_because_the_scheduler_stopped(Some("agent"));

        assert_eq!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::SchedulerUnavailable(
                "agent: the scheduler is no longer accepting requests".to_owned()
            ))
        );
    }
}
