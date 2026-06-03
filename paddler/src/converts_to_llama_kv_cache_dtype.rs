use crate::kv_cache_dtype::KvCacheDtype;
use llama_cpp_bindings::context::params::KvCacheType as LlamaKvCacheDtype;

pub trait ConvertsToLlamaKvCacheDtype {
    fn to_llama_kv_cache_dtype(self) -> LlamaKvCacheDtype;
}

impl ConvertsToLlamaKvCacheDtype for KvCacheDtype {
    fn to_llama_kv_cache_dtype(self) -> LlamaKvCacheDtype {
        match self {
            Self::F32 => LlamaKvCacheDtype::F32,
            Self::F16 => LlamaKvCacheDtype::F16,
            Self::Bf16 => LlamaKvCacheDtype::BF16,
            Self::Q80 => LlamaKvCacheDtype::Q8_0,
            Self::Q40 => LlamaKvCacheDtype::Q4_0,
            Self::Q41 => LlamaKvCacheDtype::Q4_1,
            Self::Iq4Nl => LlamaKvCacheDtype::IQ4_NL,
            Self::Q50 => LlamaKvCacheDtype::Q5_0,
            Self::Q51 => LlamaKvCacheDtype::Q5_1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ConvertsToLlamaKvCacheDtype;
    use super::KvCacheDtype;
    use super::LlamaKvCacheDtype;

    #[test]
    fn maps_each_kv_cache_dtype_to_its_llama_counterpart() {
        assert_eq!(
            KvCacheDtype::F32.to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::F32
        );
        assert_eq!(
            KvCacheDtype::F16.to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::F16
        );
        assert_eq!(
            KvCacheDtype::Bf16.to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::BF16
        );
        assert_eq!(
            KvCacheDtype::Q80.to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Q8_0
        );
        assert_eq!(
            KvCacheDtype::Q40.to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Q4_0
        );
        assert_eq!(
            KvCacheDtype::Q41.to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Q4_1
        );
        assert_eq!(
            KvCacheDtype::Iq4Nl.to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::IQ4_NL
        );
        assert_eq!(
            KvCacheDtype::Q50.to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Q5_0
        );
        assert_eq!(
            KvCacheDtype::Q51.to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Q5_1
        );
    }
}
