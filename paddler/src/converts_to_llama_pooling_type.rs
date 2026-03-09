use llama_cpp_bindings::context::params::LlamaPoolingType;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unspecified_maps_to_llama_unspecified() {
        assert_eq!(
            PoolingType::Unspecified.to_llama_pooling_type(),
            LlamaPoolingType::Unspecified
        );
    }

    #[test]
    fn none_maps_to_llama_none() {
        assert_eq!(
            PoolingType::None.to_llama_pooling_type(),
            LlamaPoolingType::None
        );
    }

    #[test]
    fn mean_maps_to_llama_mean() {
        assert_eq!(
            PoolingType::Mean.to_llama_pooling_type(),
            LlamaPoolingType::Mean
        );
    }

    #[test]
    fn cls_maps_to_llama_cls() {
        assert_eq!(
            PoolingType::Cls.to_llama_pooling_type(),
            LlamaPoolingType::Cls
        );
    }

    #[test]
    fn last_maps_to_llama_last() {
        assert_eq!(
            PoolingType::Last.to_llama_pooling_type(),
            LlamaPoolingType::Last
        );
    }

    #[test]
    fn rank_maps_to_llama_rank() {
        assert_eq!(
            PoolingType::Rank.to_llama_pooling_type(),
            LlamaPoolingType::Rank
        );
    }
}
