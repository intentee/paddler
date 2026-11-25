use std::path::PathBuf;

use tokio_tungstenite::tungstenite::Error as WebSocketError;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_model_capabilities::agent_model_capabilities_error::AgentModelCapabilitiesError;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_model_source::model_source_error::ModelSourceError;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("the decision pipeline failed")]
    DecisionPipeline(#[from] DecisionError),

    #[error("unable to resolve a model file of the desired state")]
    DesiredModelFileUnresolvable(#[from] ModelSourceError),

    #[error("the embedding pipeline failed")]
    EmbeddingPipeline(#[from] EmbeddingError),

    #[error("Local file does not exist: {path_display}", path_display = path.display())]
    LocalFileMissing { path: PathBuf },

    #[error("unable to connect to the management socket at {socket_url}")]
    ManagementSocketUnreachable {
        socket_url: String,
        #[source]
        source: Box<WebSocketError>,
    },

    #[error("the capabilities of the model cannot be established")]
    ModelCapabilities(#[from] AgentModelCapabilitiesError),

    #[error("the model failed to load")]
    ModelLoad(#[from] AgentRuntimeError),

    #[error("the text generation pipeline failed")]
    TextGenerationPipeline(#[from] TextGenerationError),
}
