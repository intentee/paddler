use std::sync::mpsc::Receiver;

use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;

use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::prepared_generation_request::PreparedGenerationRequest;

pub struct ContinuousBatchSchedulerParams<'model> {
    pub agent_shutdown: CancellationToken,
    pub batch: LlamaBatch<'static>,
    pub scheduler_message_rx: Receiver<SchedulerMessage<PreparedGenerationRequest>>,
    pub llama_context: LlamaContext<'model>,
    pub scheduler_context: ContinuousBatchSchedulerContext,
}
