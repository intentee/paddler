use paddler_messaging::kv_cache_dtype::KvCacheDtype;

use crate::kv_cache_dtype_pair_error::KvCacheDtypePairError;

pub struct KvCacheDtypePair {
    pub k_cache_dtype: KvCacheDtype,
    pub v_cache_dtype: KvCacheDtype,
}

impl KvCacheDtypePair {
    /// # Errors
    /// Returns [`KvCacheDtypePairError::QuantizedValueCacheRequiresMatchingKeyCache`] when a
    /// quantized V cache is paired with a differently typed K cache.
    pub fn validate(&self) -> Result<(), KvCacheDtypePairError> {
        if self.v_cache_dtype.is_quantized() && self.k_cache_dtype != self.v_cache_dtype {
            return Err(
                KvCacheDtypePairError::QuantizedValueCacheRequiresMatchingKeyCache {
                    k_cache_dtype: self.k_cache_dtype.clone(),
                    v_cache_dtype: self.v_cache_dtype.clone(),
                },
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::kv_cache_dtype::KvCacheDtype;

    use super::KvCacheDtypePair;
    use crate::kv_cache_dtype_pair_error::KvCacheDtypePairError;

    #[test]
    fn matching_quantized_caches_are_accepted() {
        assert!(
            KvCacheDtypePair {
                k_cache_dtype: KvCacheDtype::Q80,
                v_cache_dtype: KvCacheDtype::Q80,
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn distinct_caches_are_accepted_when_the_value_cache_is_not_quantized() {
        assert!(
            KvCacheDtypePair {
                k_cache_dtype: KvCacheDtype::Q80,
                v_cache_dtype: KvCacheDtype::F16,
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn a_quantized_value_cache_rejects_a_differently_typed_key_cache() {
        assert!(matches!(
            KvCacheDtypePair {
                k_cache_dtype: KvCacheDtype::Q80,
                v_cache_dtype: KvCacheDtype::Q40,
            }
            .validate(),
            Err(
                KvCacheDtypePairError::QuantizedValueCacheRequiresMatchingKeyCache {
                    k_cache_dtype: KvCacheDtype::Q80,
                    v_cache_dtype: KvCacheDtype::Q40,
                }
            )
        ));
    }
}
