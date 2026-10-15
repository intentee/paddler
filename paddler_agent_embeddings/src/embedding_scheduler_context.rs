use paddler_inference_parameters::pooling_type::PoolingType;

pub struct EmbeddingSchedulerContext {
    pub agent_name: Option<String>,
    pub desired_slots_total: u16,
    pub n_batch: usize,
    pub pooling_type: PoolingType,
}
