use llama_cpp_bindings::context::params::KvCacheType as LlamaKvCacheType;
use paddler_types::kv_cache_type::KvCacheType;

pub trait ConvertsToLlamaKvCacheType {
    fn to_llama_kv_cache_type(self) -> LlamaKvCacheType;
}

impl ConvertsToLlamaKvCacheType for KvCacheType {
    fn to_llama_kv_cache_type(self) -> LlamaKvCacheType {
        match self {
            Self::F16 => LlamaKvCacheType::F16,
            Self::Q4_0 => LlamaKvCacheType::Q4_0,
            Self::Q8_0 => LlamaKvCacheType::Q8_0,
        }
    }
}
