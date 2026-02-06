use llama_cpp_2::context::params::LlamaPoolingType;
use paddler_types::pooling_type::PoolingType;

pub trait ConvertsToLlamaPoolingType {
    fn to_llama_pooling_type(self) -> LlamaPoolingType;
}

impl ConvertsToLlamaPoolingType for PoolingType {
    fn to_llama_pooling_type(self) -> LlamaPoolingType {
        match self {
            PoolingType::Unspecified => LlamaPoolingType::Unspecified,
            PoolingType::None => LlamaPoolingType::None,
            PoolingType::Mean => LlamaPoolingType::Mean,
            PoolingType::Cls => LlamaPoolingType::Cls,
            PoolingType::Last => LlamaPoolingType::Last,
            PoolingType::Rank => LlamaPoolingType::Rank,
        }
    }
}
