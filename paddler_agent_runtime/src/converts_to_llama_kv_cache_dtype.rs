use llama_cpp_bindings::context::params::KvCacheType as LlamaKvCacheDtype;

pub trait ConvertsToLlamaKvCacheDtype {
    fn to_llama_kv_cache_dtype(self) -> LlamaKvCacheDtype;
}
