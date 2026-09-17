use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum KvCacheDtype {
    F32,
    F16,
    #[serde(rename = "BF16")]
    Bf16,
    #[serde(rename = "Q8_0")]
    Q80,
    #[serde(rename = "Q4_0")]
    Q40,
    #[serde(rename = "Q4_1")]
    Q41,
    #[serde(rename = "IQ4_NL")]
    Iq4Nl,
    #[serde(rename = "Q5_0")]
    Q50,
    #[serde(rename = "Q5_1")]
    Q51,
}

impl KvCacheDtype {
    #[must_use]
    pub const fn is_quantized(&self) -> bool {
        match self {
            Self::F32 | Self::F16 | Self::Bf16 => false,
            Self::Q80 | Self::Q40 | Self::Q41 | Self::Iq4Nl | Self::Q50 | Self::Q51 => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KvCacheDtype;

    #[test]
    fn floating_point_cache_types_are_not_quantized() {
        assert!(!KvCacheDtype::F32.is_quantized());
        assert!(!KvCacheDtype::F16.is_quantized());
        assert!(!KvCacheDtype::Bf16.is_quantized());
    }

    #[test]
    fn block_quantized_cache_types_are_quantized() {
        assert!(KvCacheDtype::Q80.is_quantized());
        assert!(KvCacheDtype::Q40.is_quantized());
        assert!(KvCacheDtype::Q41.is_quantized());
        assert!(KvCacheDtype::Iq4Nl.is_quantized());
        assert!(KvCacheDtype::Q50.is_quantized());
        assert!(KvCacheDtype::Q51.is_quantized());
    }
}
