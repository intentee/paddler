use std::io;
use std::num::TryFromIntError;
use std::path::PathBuf;

use llama_cpp_backend::backend_error::BackendError;
use llama_cpp_context::context_error::ContextError;
use llama_cpp_model::model_error::ModelError;

use paddler_messaging::inference_capability::InferenceCapability;

#[derive(Debug, thiserror::Error)]
pub enum AgentRuntimeError {
    #[error("unable to determine how many threads this machine runs in parallel")]
    AvailableParallelismUnknown(#[source] io::Error),

    #[error("unable to initialize the llama.cpp backend")]
    BackendInitializationFailed(#[source] BackendError),

    #[error("unable to create the llama.cpp context")]
    ContextCreationFailed(#[source] ContextError),

    #[error("a {inference_capability:?} context is already resident in VRAM")]
    LlamaContextAlreadyResident {
        inference_capability: InferenceCapability,
    },

    #[error("unable to read the model file {model_path}")]
    ModelFilePrefetchFailed {
        model_path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("unable to load the model from {model_path}")]
    ModelLoadFailed {
        model_path: PathBuf,
        #[source]
        source: ModelError,
    },

    #[error("unable to tell which decision type the loaded model is")]
    ModelDecisionTypeUnreadable(#[source] ModelError),

    #[error("unable to read the metadata of the loaded model")]
    ModelMetadataUnreadable(#[source] ModelError),

    #[error("model weights are already resident in VRAM")]
    ModelWeightsAlreadyResident,

    #[error("a multimodal projection is already resident in VRAM")]
    MultimodalProjectionAlreadyResident,

    #[error("the machine runs more threads in parallel than llama.cpp can address")]
    ThreadCountOutOfRange(#[source] TryFromIntError),

    #[error("unable to assemble the warm-up batch")]
    WarmUpBatchAssemblyFailed(#[source] ContextError),

    #[error("llama.cpp rejected the warm-up batch")]
    WarmUpDecodeFailed(#[source] ContextError),
}
