use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::task::TaskTracker;

use paddler_agent_embeddings::embedding_batch_preparer::EmbeddingBatchPreparer;
use paddler_agent_embeddings::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
use paddler_agent_embeddings::sequence_context_limit::SequenceContextLimit;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::embedding_batch_generation::EmbeddingBatchGeneration;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_model_card_fixture::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

pub struct EmbeddingRequestPreparerUnderTest {
    pub llama_runtime: LlamaRuntime,
    pub request_preparer: Arc<SchedulerRequestPreparer<EmbeddingBatchPreparer>>,
    pub scheduler_message_rx: UnboundedReceiver<SchedulerMessage<PreparedEmbeddingBatchRequest>>,
}

impl EmbeddingRequestPreparerUnderTest {
    /// # Errors
    ///
    /// Returns an error when the nomic embedding model is not in the Hugging Face cache or cannot load.
    pub fn load() -> Result<Self> {
        let model_path = nomic_embed_text_v1_5()
            .cached_path()
            .context("the nomic embedding model must be in the Hugging Face cache")?;
        let llama_runtime = LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
            .context("the nomic embedding model must load")?;
        let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

        Ok(Self {
            request_preparer: Arc::new(SchedulerRequestPreparer {
                agent_name: llama_runtime.inference_runtime_context.agent_name.clone(),
                preparation: EmbeddingBatchPreparer {
                    loaded_llama_model: llama_runtime.loaded_llama_model.clone(),
                    n_batch: llama_runtime
                        .model_runtime_parameters
                        .n_batch
                        .tokens_usize(),
                    sequence_context_limit: SequenceContextLimit::Unlimited,
                },
                preparation_tasks: TaskTracker::new(),
                scheduler_message_tx,
            }),
            llama_runtime,
            scheduler_message_rx,
        })
    }

    #[must_use]
    pub fn drained_scheduler_messages(
        mut self,
    ) -> Vec<SchedulerMessage<PreparedEmbeddingBatchRequest>> {
        let mut scheduler_messages = Vec::new();

        while let Ok(scheduler_message) = self.scheduler_message_rx.try_recv() {
            scheduler_messages.push(scheduler_message);
        }

        scheduler_messages
    }

    #[must_use]
    pub fn embedding_batch(
        &self,
        input_batch: Vec<EmbeddingInputDocument>,
    ) -> EmbeddingBatchGeneration {
        EmbeddingBatchGeneration::new(
            input_batch,
            self.llama_runtime
                .inference_runtime_context
                .slot_aggregated_status
                .clone(),
        )
    }
}
