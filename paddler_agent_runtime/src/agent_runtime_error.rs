use std::io;
use std::num::TryFromIntError;
use std::path::PathBuf;

use llama_cpp_bindings::LlamaCppError;
use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::error::DecodeError;
use llama_cpp_bindings::error::LlamaContextLoadError;
use llama_cpp_bindings::error::LlamaModelLoadError;
use llama_cpp_bindings::error::MetaValError;

use paddler_messaging::inference_capability::InferenceCapability;

#[derive(Debug, thiserror::Error)]
pub enum AgentRuntimeError {
    #[error("unable to determine how many threads this machine runs in parallel: {0}")]
    AvailableParallelismUnknown(#[source] io::Error),

    #[error("unable to initialize the llama.cpp backend: {0}")]
    BackendInitializationFailed(#[source] LlamaCppError),

    #[error("unable to create the llama.cpp context: {0}")]
    ContextCreationFailed(#[source] LlamaContextLoadError),

    #[error("a {inference_capability:?} context is already resident in VRAM")]
    LlamaContextAlreadyResident {
        inference_capability: InferenceCapability,
    },

    #[error("unable to load the model from {model_path}: {source}")]
    ModelLoadFailed {
        model_path: PathBuf,
        #[source]
        source: LlamaModelLoadError,
    },

    #[error("unable to read the metadata of the loaded model: {0}")]
    ModelMetadataUnreadable(#[source] MetaValError),

    #[error("model weights are already resident in VRAM")]
    ModelWeightsAlreadyResident,

    #[error("a multimodal projection is already resident in VRAM")]
    MultimodalProjectionAlreadyResident,

    #[error("the machine runs more threads in parallel than llama.cpp can address: {0}")]
    ThreadCountOutOfRange(#[source] TryFromIntError),

    #[error("unable to assemble the warm-up batch: {0}")]
    WarmUpBatchAssemblyFailed(#[source] BatchAddError),

    #[error("llama.cpp rejected the warm-up batch: {0}")]
    WarmUpDecodeFailed(#[source] DecodeError),
}
