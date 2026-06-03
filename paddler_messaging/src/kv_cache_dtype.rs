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
