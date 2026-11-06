use llama_cpp_bindings::model::LlamaModel;

pub struct ContinuousBatchSchedulerContext<'model> {
    pub agent_name: Option<String>,
    pub desired_slots_total: u16,
    pub model: &'model LlamaModel,
    pub sequence_context_size: u32,
}
