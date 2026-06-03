use crate::pooling_type::PoolingType;
use llama_cpp_bindings::context::params::LlamaPoolingType;

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

#[cfg(test)]
mod tests {
    use super::ConvertsToLlamaPoolingType;
    use super::LlamaPoolingType;
    use super::PoolingType;

    #[test]
    fn converts_unspecified() {
        assert_eq!(
            PoolingType::Unspecified.to_llama_pooling_type(),
            LlamaPoolingType::Unspecified
        );
    }

    #[test]
    fn converts_none() {
        assert_eq!(
            PoolingType::None.to_llama_pooling_type(),
            LlamaPoolingType::None
        );
    }

    #[test]
    fn converts_mean() {
        assert_eq!(
            PoolingType::Mean.to_llama_pooling_type(),
            LlamaPoolingType::Mean
        );
    }

    #[test]
    fn converts_cls() {
        assert_eq!(
            PoolingType::Cls.to_llama_pooling_type(),
            LlamaPoolingType::Cls
        );
    }

    #[test]
    fn converts_last() {
        assert_eq!(
            PoolingType::Last.to_llama_pooling_type(),
            LlamaPoolingType::Last
        );
    }

    #[test]
    fn converts_rank() {
        assert_eq!(
            PoolingType::Rank.to_llama_pooling_type(),
            LlamaPoolingType::Rank
        );
    }
}
