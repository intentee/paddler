use log::warn;
use tokio::sync::mpsc;

use paddler_agent_runtime::scheduler_command::SchedulerCommand;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;

use crate::active_embedding_request::ActiveEmbeddingRequest;
use crate::embedding_error::EmbeddingError;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::embedding_request_continuation::EmbeddingRequestContinuation;
use crate::unembeddable_embedding_document::UnembeddableEmbeddingDocument;

pub struct PreparedEmbeddingBatchRequest {
    pub generate_embedding_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_embedding_tx: mpsc::UnboundedSender<EmbeddingResult>,
    pub inputs: Vec<EmbeddingInputTokenized>,
    pub normalization_method: EmbeddingNormalizationMethod,
    pub unembeddable_documents: Vec<UnembeddableEmbeddingDocument>,
    pub slot_guard: SlotGuard,
}

impl PreparedEmbeddingBatchRequest {
    #[must_use]
    pub fn admit(self, agent_name: Option<&str>) -> EmbeddingRequestContinuation {
        let Self {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            inputs,
            normalization_method,
            unembeddable_documents,
            slot_guard,
        } = self;

        for unembeddable_document in unembeddable_documents {
            warn!("{agent_name:?}: skipped an unembeddable document: {unembeddable_document:?}");

            if let Err(send_error) =
                generated_embedding_tx.send(unembeddable_document.into_embedding_result())
            {
                EmbeddingError::ClientDisconnected(send_error)
                    .report(agent_name, &generated_embedding_tx);

                return EmbeddingRequestContinuation::Concluded;
            }
        }

        ActiveEmbeddingRequest {
            embeddings_emitted: 0,
            generate_embedding_stop_rx,
            generated_embedding_tx,
            normalization_method,
            pending_inputs: inputs.into(),
            slot_guard,
        }
        .continue_unless_every_input_is_embedded(agent_name)
    }
}

impl SchedulerCommand for PreparedEmbeddingBatchRequest {
    fn reject_because_the_scheduler_stopped(self, agent_name: Option<&str>) {
        EmbeddingError::SchedulerUnavailable.report(agent_name, &self.generated_embedding_tx);
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::num::NonZeroU16;
    use std::sync::Arc;

    use llama_cpp_bindings::token::LlamaToken;
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::error::TryRecvError;

    use paddler_agent_runtime::scheduler_command::SchedulerCommand as _;
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_status::slot_guard::SlotGuard;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

    use super::PreparedEmbeddingBatchRequest;
    use crate::embedding_input_tokenized::EmbeddingInputTokenized;
    use crate::embedding_request_continuation::EmbeddingRequestContinuation;
    use crate::unembeddable_embedding_document::UnembeddableEmbeddingDocument;

    fn oversized_document_details() -> OversizedEmbeddingDocumentDetails {
        OversizedEmbeddingDocumentDetails {
            document_tokens: 4096,
            n_batch: 2048,
            source_document_id: "oversized".to_owned(),
        }
    }

    fn prepared_request(
        generated_embedding_tx: mpsc::UnboundedSender<EmbeddingResult>,
        inputs: Vec<EmbeddingInputTokenized>,
    ) -> PreparedEmbeddingBatchRequest {
        let (_generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

        PreparedEmbeddingBatchRequest {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            inputs,
            normalization_method: EmbeddingNormalizationMethod::None,
            unembeddable_documents: vec![UnembeddableEmbeddingDocument::ExceedsBatchSize(
                oversized_document_details(),
            )],
            slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1)), NonZeroU16::MIN),
        }
    }

    #[test]
    fn rejects_an_embedding_batch_when_the_scheduler_stopped() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();

        prepared_request(generated_embedding_tx, Vec::new())
            .reject_because_the_scheduler_stopped(Some("agent"));

        assert_eq!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::SchedulerUnavailable(
                "agent: the scheduler is no longer accepting requests".to_owned()
            ))
        );
    }

    #[test]
    fn a_batch_of_only_oversized_documents_reports_them_and_produces_no_embeddings() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();

        assert_eq!(
            discriminant(&prepared_request(generated_embedding_tx, Vec::new()).admit(None)),
            discriminant(&EmbeddingRequestContinuation::Concluded)
        );
        assert_eq!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::DocumentExceedsBatchSize(
                oversized_document_details()
            ))
        );
        assert_eq!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::NoEmbeddingsProduced)
        );
        assert_eq!(
            generated_embedding_rx.try_recv(),
            Err(TryRecvError::Disconnected)
        );
    }

    #[test]
    fn a_client_that_stopped_listening_concludes_before_its_documents_are_embedded() {
        let (generated_embedding_tx, generated_embedding_rx) = mpsc::unbounded_channel();

        drop(generated_embedding_rx);

        assert_eq!(
            discriminant(
                &prepared_request(
                    generated_embedding_tx,
                    vec![EmbeddingInputTokenized {
                        id: "document".to_owned(),
                        tokens: vec![LlamaToken::new(0)],
                    }],
                )
                .admit(None)
            ),
            discriminant(&EmbeddingRequestContinuation::Concluded)
        );
    }

    #[test]
    fn a_batch_with_documents_to_embed_stays_open_after_reporting_its_oversized_documents() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();
        let _continuation = prepared_request(
            generated_embedding_tx,
            vec![EmbeddingInputTokenized {
                id: "document".to_owned(),
                tokens: vec![LlamaToken::new(0)],
            }],
        )
        .admit(None);

        assert_eq!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::DocumentExceedsBatchSize(
                oversized_document_details()
            ))
        );
        assert_eq!(generated_embedding_rx.try_recv(), Err(TryRecvError::Empty));
    }
}
