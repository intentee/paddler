use llama_cpp_bindings::context::params::KvCacheType as LlamaKvCacheDtype;
use paddler_types::kv_cache_dtype::KvCacheDtype;

pub trait ConvertsToLlamaKvCacheDtype {
    fn to_llama_kv_cache_dtype(self) -> LlamaKvCacheDtype;
}

impl ConvertsToLlamaKvCacheDtype for KvCacheDtype {
    fn to_llama_kv_cache_dtype(self) -> LlamaKvCacheDtype {
        match self {
            Self::F32 => LlamaKvCacheDtype::F32,
            Self::F16 => LlamaKvCacheDtype::F16,
            Self::BF16 => LlamaKvCacheDtype::BF16,
            Self::Q8_0 => LlamaKvCacheDtype::Q8_0,
            Self::Q4_0 => LlamaKvCacheDtype::Q4_0,
            Self::Q4_1 => LlamaKvCacheDtype::Q4_1,
            Self::IQ4_NL => LlamaKvCacheDtype::IQ4_NL,
            Self::Q5_0 => LlamaKvCacheDtype::Q5_0,
            Self::Q5_1 => LlamaKvCacheDtype::Q5_1,
        }
    }
}
