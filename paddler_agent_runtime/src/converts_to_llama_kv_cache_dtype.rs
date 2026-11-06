use llama_cpp_bindings::context::kv_cache_type::KvCacheType as LlamaKvCacheDtype;

pub trait ConvertsToLlamaKvCacheDtype {
    fn to_llama_kv_cache_dtype(self) -> LlamaKvCacheDtype;
}
