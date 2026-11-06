use llama_cpp_bindings::context::kv_cache_type::KvCacheType as LlamaKvCacheDtype;

use paddler_inference_parameters::kv_cache_dtype::KvCacheDtype;

use crate::converts_to_llama_kv_cache_dtype::ConvertsToLlamaKvCacheDtype;

pub struct AgentKvCacheDtype(pub KvCacheDtype);

impl ConvertsToLlamaKvCacheDtype for AgentKvCacheDtype {
    fn to_llama_kv_cache_dtype(self) -> LlamaKvCacheDtype {
        match self.0 {
            KvCacheDtype::F32 => LlamaKvCacheDtype::F32,
            KvCacheDtype::F16 => LlamaKvCacheDtype::F16,
            KvCacheDtype::Bf16 => LlamaKvCacheDtype::Bf16,
            KvCacheDtype::Q80 => LlamaKvCacheDtype::Q80,
            KvCacheDtype::Q40 => LlamaKvCacheDtype::Q40,
            KvCacheDtype::Q41 => LlamaKvCacheDtype::Q41,
            KvCacheDtype::Iq4Nl => LlamaKvCacheDtype::Iq4Nl,
            KvCacheDtype::Q50 => LlamaKvCacheDtype::Q50,
            KvCacheDtype::Q51 => LlamaKvCacheDtype::Q51,
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::context::kv_cache_type::KvCacheType as LlamaKvCacheDtype;

    use paddler_inference_parameters::kv_cache_dtype::KvCacheDtype;

    use super::AgentKvCacheDtype;
    use crate::converts_to_llama_kv_cache_dtype::ConvertsToLlamaKvCacheDtype;

    #[test]
    fn maps_each_kv_cache_dtype_to_its_llama_counterpart() {
        assert_eq!(
            AgentKvCacheDtype(KvCacheDtype::F32).to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::F32
        );
        assert_eq!(
            AgentKvCacheDtype(KvCacheDtype::F16).to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::F16
        );
        assert_eq!(
            AgentKvCacheDtype(KvCacheDtype::Bf16).to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Bf16
        );
        assert_eq!(
            AgentKvCacheDtype(KvCacheDtype::Q80).to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Q80
        );
        assert_eq!(
            AgentKvCacheDtype(KvCacheDtype::Q40).to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Q40
        );
        assert_eq!(
            AgentKvCacheDtype(KvCacheDtype::Q41).to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Q41
        );
        assert_eq!(
            AgentKvCacheDtype(KvCacheDtype::Iq4Nl).to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Iq4Nl
        );
        assert_eq!(
            AgentKvCacheDtype(KvCacheDtype::Q50).to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Q50
        );
        assert_eq!(
            AgentKvCacheDtype(KvCacheDtype::Q51).to_llama_kv_cache_dtype(),
            LlamaKvCacheDtype::Q51
        );
    }
}
