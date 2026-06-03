use llama_cpp_bindings::context::params::LlamaPoolingType;
use paddler_messaging::pooling_type::PoolingType;

use crate::converts_to_llama_pooling_type::ConvertsToLlamaPoolingType;

pub struct AgentPoolingType(pub PoolingType);

impl ConvertsToLlamaPoolingType for AgentPoolingType {
    fn to_llama_pooling_type(self) -> LlamaPoolingType {
        match self.0 {
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
    use llama_cpp_bindings::context::params::LlamaPoolingType;
    use paddler_messaging::pooling_type::PoolingType;

    use super::AgentPoolingType;
    use crate::converts_to_llama_pooling_type::ConvertsToLlamaPoolingType;

    #[test]
    fn converts_unspecified() {
        assert_eq!(
            AgentPoolingType(PoolingType::Unspecified).to_llama_pooling_type(),
            LlamaPoolingType::Unspecified
        );
    }

    #[test]
    fn converts_none() {
        assert_eq!(
            AgentPoolingType(PoolingType::None).to_llama_pooling_type(),
            LlamaPoolingType::None
        );
    }

    #[test]
    fn converts_mean() {
        assert_eq!(
            AgentPoolingType(PoolingType::Mean).to_llama_pooling_type(),
            LlamaPoolingType::Mean
        );
    }

    #[test]
    fn converts_cls() {
        assert_eq!(
            AgentPoolingType(PoolingType::Cls).to_llama_pooling_type(),
            LlamaPoolingType::Cls
        );
    }

    #[test]
    fn converts_last() {
        assert_eq!(
            AgentPoolingType(PoolingType::Last).to_llama_pooling_type(),
            LlamaPoolingType::Last
        );
    }

    #[test]
    fn converts_rank() {
        assert_eq!(
            AgentPoolingType(PoolingType::Rank).to_llama_pooling_type(),
            LlamaPoolingType::Rank
        );
    }
}
