use std::num::TryFromIntError;
use std::path::PathBuf;

use llama_cpp_bindings::gguf_context_error::GgufContextError;

#[derive(Debug, thiserror::Error)]
pub enum PointerHeadError {
    #[error("unable to read the pointer head file {path}: {source}")]
    FileUnreadable {
        path: PathBuf,
        #[source]
        source: GgufContextError,
    },

    #[error("the pointer head dimension {head_dim} does not fit sixteen bits: {source}")]
    HeadDimensionOutOfRange {
        head_dim: i64,
        #[source]
        source: TryFromIntError,
    },

    #[error(
        "the pointer head declares the input normalization {declared}, which is not NFC, NFD, NFKC, NFKD or None"
    )]
    InputNormalizationUnknown { declared: String },

    #[error("unable to read the pointer head key {key}: {source}")]
    KeyUnreadable {
        key: String,
        #[source]
        source: GgufContextError,
    },

    #[error("the pointer head temperature {temperature} is not a finite positive number")]
    TemperatureOutOfRange { temperature: f32 },

    #[error("the pointer head tensor {tensor} has the shape {actual:?} instead of {expected:?}")]
    TensorShapeMismatch {
        tensor: String,
        expected: [i64; 4],
        actual: [i64; 4],
    },

    #[error("unable to read the pointer head tensor {tensor}: {source}")]
    TensorUnreadable {
        tensor: String,
        #[source]
        source: GgufContextError,
    },
}
