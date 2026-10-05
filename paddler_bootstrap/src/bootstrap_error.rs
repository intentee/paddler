use std::io;
use std::net::SocketAddr;

use anyhow::Error as AnyhowError;
use thiserror::Error;

use paddler_state_database::state_database_error::StateDatabaseError;

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
    #[error("A Paddler service stopped with an error")]
    ServiceRunFailed {
        #[source]
        source: AnyhowError,
    },
    #[error("A Paddler service thread panicked")]
    ServiceThreadPanicked,
    #[error("The statsd reporting interval must be longer than zero")]
    StatsdReportingIntervalIsZero,
    #[error("Unable to read the balancer desired state from the state database")]
    StateDatabaseReadFailed {
        #[source]
        source: StateDatabaseError,
    },
    #[cfg(feature = "web_admin_panel")]
    #[error("Unable to bind the web admin panel service to {addr}")]
    WebAdminPanelBindFailed {
        addr: SocketAddr,
        #[source]
        source: io::Error,
    },
}
