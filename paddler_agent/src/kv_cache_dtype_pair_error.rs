use paddler_messaging::kv_cache_dtype::KvCacheDtype;

#[derive(Debug, thiserror::Error)]
pub enum KvCacheDtypePairError {
    #[error(
        "a quantized V cache forces flash attention on, and flash attention requires both caches to use the same type, but K is {k_cache_dtype:?} and V is {v_cache_dtype:?}"
    )]
    QuantizedValueCacheRequiresMatchingKeyCache {
        k_cache_dtype: KvCacheDtype,
        v_cache_dtype: KvCacheDtype,
    },
}
