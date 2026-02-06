use llama_cpp_2::context::params::LlamaPoolingType;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[repr(i8)]
pub enum PoolingType {
    Unspecified = -1,
    None = 0,
    Mean = 1,
    Cls = 2,
    Last = 3,
    Rank = 4,
}

impl From<PoolingType> for LlamaPoolingType {
    fn from(pooling_type: PoolingType) -> LlamaPoolingType {
        match pooling_type {
            PoolingType::Unspecified => LlamaPoolingType::Unspecified,
            PoolingType::None => LlamaPoolingType::None,
            PoolingType::Mean => LlamaPoolingType::Mean,
            PoolingType::Cls => LlamaPoolingType::Cls,
            PoolingType::Last => LlamaPoolingType::Last,
            PoolingType::Rank => LlamaPoolingType::Rank,
        }
    }
}
