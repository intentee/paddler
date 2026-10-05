use std::sync::mpsc::Receiver;

use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use tokio_util::sync::CancellationToken;

use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;

pub struct ContinuousBatchSchedulerParams<'model> {
    pub agent_shutdown: CancellationToken,
    pub batch: LlamaBatch<'static>,
    pub command_rx: Receiver<ContinuousBatchSchedulerCommand>,
    pub llama_context: LlamaContext<'model>,
    pub scheduler_context: ContinuousBatchSchedulerContext,
}
