from paddler_client.kv_cache_dtype import KvCacheDtype


def test_kv_cache_dtype_values() -> None:
    assert KvCacheDtype.F32.value == "F32"
    assert KvCacheDtype.F16.value == "F16"
    assert KvCacheDtype.BF16.value == "BF16"
    assert KvCacheDtype.Q8_0.value == "Q8_0"
    assert KvCacheDtype.Q4_0.value == "Q4_0"
    assert KvCacheDtype.Q4_1.value == "Q4_1"
    assert KvCacheDtype.IQ4_NL.value == "IQ4_NL"
    assert KvCacheDtype.Q5_0.value == "Q5_0"
    assert KvCacheDtype.Q5_1.value == "Q5_1"
