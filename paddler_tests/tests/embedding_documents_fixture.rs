#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;
use std::sync::Arc;

use tokio::sync::mpsc;

use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_tests::numbered_embedding_documents::numbered_embedding_documents;

pub struct EmbeddingDocumentsFixture {
    pub agent_request: AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>,
    pub generate_embedding_stop_tx: mpsc::UnboundedSender<()>,
    pub generated_embedding_rx: mpsc::UnboundedReceiver<EmbeddingResult>,
}

impl EmbeddingDocumentsFixture {
    pub fn embedded_one_at_a_time(
        document_count: usize,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Self {
        let (generated_embedding_tx, generated_embedding_rx) = mpsc::unbounded_channel();
        let (generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

        Self {
            agent_request: AgentRequest {
                params: GenerateEmbeddingBatchParams {
                    input_batch: numbered_embedding_documents(document_count),
                    normalization_method: EmbeddingNormalizationMethod::None,
                },
                response_tx: generated_embedding_tx,
                slot_guard: SlotGuard::new(slot_aggregated_status, NonZeroU16::MIN),
                stop_rx: generate_embedding_stop_rx,
            },
            generate_embedding_stop_tx,
            generated_embedding_rx,
        }
    }

    pub async fn collect_results(
        mut generated_embedding_rx: mpsc::UnboundedReceiver<EmbeddingResult>,
    ) -> Vec<EmbeddingResult> {
        let mut embedding_results = Vec::new();

        while let Some(embedding_result) = generated_embedding_rx.recv().await {
            embedding_results.push(embedding_result);
        }

        embedding_results
    }
}
