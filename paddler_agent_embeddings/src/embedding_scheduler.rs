use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvError;

use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use log::info;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;

use crate::embedding_batch_processor::EmbeddingBatchProcessor;
use crate::embedding_scheduler_context::EmbeddingSchedulerContext;
use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;

pub struct EmbeddingScheduler<'model> {
    pub agent_shutdown: CancellationToken,
    pub batch: LlamaBatch<'static>,
    pub scheduler_message_rx: Receiver<SchedulerMessage<PreparedEmbeddingBatchRequest>>,
    pub llama_context: LlamaContext<'model>,
    pub scheduler_context: EmbeddingSchedulerContext,
}

impl EmbeddingScheduler<'_> {
    pub fn run(&mut self) {
        info!(
            "{:?}: embedding scheduler started",
            self.scheduler_context.agent_name
        );

        while !self.agent_shutdown.is_cancelled() {
            match self.scheduler_message_rx.recv() {
                Ok(SchedulerMessage::Command(request)) => {
                    let generated_embedding_tx = request.generated_embedding_tx.clone();

                    if let Err(rejection) = (EmbeddingBatchProcessor {
                        batch: &mut self.batch,
                        llama_context: &mut self.llama_context,
                        scheduler_context: &self.scheduler_context,
                    })
                    .process_embedding_batch(request)
                    {
                        rejection.report(
                            self.scheduler_context.agent_name.as_deref(),
                            &generated_embedding_tx,
                        );
                    }
                }
                Ok(SchedulerMessage::Shutdown) | Err(RecvError) => break,
            }
        }

        self.llama_context.synchronize();
        self.llama_context.detach_threadpool();

        info!(
            "{:?}: embedding scheduler stopped",
            self.scheduler_context.agent_name
        );
    }
}
