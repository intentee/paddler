use std::io;
use std::net::SocketAddr;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("Unable to bind the OpenAI compatibility service to {addr}")]
    CompatOpenAIBindFailed {
        addr: SocketAddr,
        #[source]
        source: io::Error,
    },
    #[error("Unable to bind the inference service to {addr}")]
    InferenceBindFailed {
        addr: SocketAddr,
        #[source]
        source: io::Error,
    },
    #[error("Unable to bind the management service to {addr}")]
    ManagementBindFailed {
        addr: SocketAddr,
        #[source]
        source: io::Error,
    },
    #[error("Unable to read the balancer desired state from the state database")]
    StateDatabaseReadFailed {
        #[source]
        source: anyhow::Error,
    },
    #[cfg(feature = "web_admin_panel")]
    #[error("Unable to bind the web admin panel service to {addr}")]
    WebAdminPanelBindFailed {
        addr: SocketAddr,
        #[source]
        source: io::Error,
    },
}
