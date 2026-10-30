use std::mem::take;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use log::info;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;

use crate::active_embedding_request::ActiveEmbeddingRequest;
use crate::embedding_batch_processor::EmbeddingBatchProcessor;
use crate::embedding_error::EmbeddingError;
use crate::embedding_normalization::EmbeddingNormalization;
use crate::embedding_scheduler_context::EmbeddingSchedulerContext;
use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;

pub struct EmbeddingScheduler<'model> {
    pub active_requests: Vec<ActiveEmbeddingRequest>,
    pub agent_shutdown: CancellationToken,
    pub batch: LlamaBatch<'static>,
    pub llama_context: ResidentLlamaContext<'model>,
    pub scheduler_context: EmbeddingSchedulerContext,
    pub scheduler_inbox: SchedulerInbox<PreparedEmbeddingBatchRequest>,
}

impl EmbeddingScheduler<'_> {
    pub fn run(&mut self) {
        info!(
            "{:?}: embedding scheduler started",
            self.scheduler_context.agent_name
        );

        while !self.agent_shutdown.is_cancelled() {
            if self.active_requests.is_empty() {
                if self.scheduler_inbox.shutdown_requested {
                    break;
                }

                self.wait_for_next_scheduler_message();
            } else {
                self.accept_new_scheduler_messages();
            }

            let active_requests = take(&mut self.active_requests);

            self.active_requests = EmbeddingBatchProcessor {
                batch: &mut self.batch,
                llama_context: &mut self.llama_context,
                normalization: EmbeddingNormalization,
                scheduler_context: &self.scheduler_context,
            }
            .embed_next_batch(active_requests);
        }

        self.reject_unfinished_requests();
        self.llama_context.synchronize();
        self.llama_context.detach_threadpool();

        info!(
            "{:?}: embedding scheduler stopped",
            self.scheduler_context.agent_name
        );
    }

    fn accept_new_scheduler_messages(&mut self) {
        while let Some(request) = self.scheduler_inbox.take_next_waiting_command() {
            self.admit(request);
        }
    }

    fn admit(&mut self, request: PreparedEmbeddingBatchRequest) {
        self.active_requests
            .extend(request.admit(self.scheduler_context.agent_name.as_deref()));
    }

    fn reject_unfinished_requests(&mut self) {
        let agent_name = self.scheduler_context.agent_name.as_deref();

        for active_request in self.active_requests.drain(..) {
            EmbeddingError::SchedulerUnavailable
                .report(agent_name, &active_request.generated_embedding_tx);
        }

        self.scheduler_inbox.reject_waiting_commands(agent_name);
    }

    fn wait_for_next_scheduler_message(&mut self) {
        if let Some(request) = self.scheduler_inbox.wait_for_next_command() {
            self.admit(request);
        }
    }
}
