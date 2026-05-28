use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[expect(
    non_camel_case_types,
    reason = "variant names mirror ggml type identifiers (e.g. GGML_TYPE_IQ4_NL) for parity with llama.cpp's --cache-type-k/-v"
)]
pub enum KvCacheDtype {
    F32,
    F16,
    BF16,
    Q8_0,
    Q4_0,
    Q4_1,
    IQ4_NL,
    Q5_0,
    Q5_1,
}
