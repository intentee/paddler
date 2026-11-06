use paddler_inference_parameters::pooling_type::PoolingType;

pub struct EmbeddingSchedulerContext {
    pub agent_name: Option<String>,
    pub pooling_type: PoolingType,
}
