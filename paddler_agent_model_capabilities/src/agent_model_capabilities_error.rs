use std::ffi::NulError;
use std::path::PathBuf;

use llama_cpp_model::model_error::ModelError;
use llama_cpp_model_capabilities::model_capabilities_error::ModelCapabilitiesError;
use llama_cpp_mtmd::mtmd_error::MtmdError;
use thiserror::Error;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;

#[derive(Debug, Error)]
pub enum AgentModelCapabilitiesError {
    #[error("the multimodal projection cannot claim the VRAM")]
    AgentRuntime(#[from] AgentRuntimeError),
    #[error("llama.cpp cannot detect what the model serves")]
    CapabilitiesUndetectable(#[source] ModelCapabilitiesError),
    #[error("the chat template override contains a NUL byte")]
    ChatTemplateContainsNul(#[source] NulError),
    #[error("the chat template does not compile")]
    ChatTemplateDoesNotCompile(#[source] ModelError),
    #[error("the embedded chat template cannot be read")]
    EmbeddedChatTemplateUnreadable(#[source] ModelError),
    #[error("the multimodal projection {multimodal_projection_path:?} cannot be loaded")]
    MultimodalProjectionLoadFailed {
        multimodal_projection_path: PathBuf,
        #[source]
        source: MtmdError,
    },
}
