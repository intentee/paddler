use llama_cpp_bindings::model::LlamaModel;

use paddler_inference_parameters::batch_size::BatchSize;

pub struct ContinuousBatchSchedulerContext<'model> {
    pub agent_name: Option<String>,
    pub desired_slots_total: u16,
    pub model: &'model LlamaModel,
    pub n_batch: BatchSize,
    pub sequence_context_size: u32,
}
