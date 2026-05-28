use llama_cpp_bindings::context::params::LlamaPoolingType;
use crate::pooling_type::PoolingType;

pub trait ConvertsToLlamaPoolingType {
    fn to_llama_pooling_type(self) -> LlamaPoolingType;
}

impl ConvertsToLlamaPoolingType for PoolingType {
    fn to_llama_pooling_type(self) -> LlamaPoolingType {
        match self {
            Self::Unspecified => LlamaPoolingType::Unspecified,
            Self::None => LlamaPoolingType::None,
            Self::Mean => LlamaPoolingType::Mean,
            Self::Cls => LlamaPoolingType::Cls,
            Self::Last => LlamaPoolingType::Last,
            Self::Rank => LlamaPoolingType::Rank,
        }
    }
}
