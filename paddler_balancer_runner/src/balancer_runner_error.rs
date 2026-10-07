use std::io;
use std::net::SocketAddr;

use thiserror::Error;

use paddler_service_thread::service_thread_error::ServiceThreadError;
use paddler_state_database::state_database_error::StateDatabaseError;

#[derive(Debug, Error)]
pub enum BalancerRunnerError {
    #[error("Unable to bind the OpenAI compatibility service to {addr}")]
    CompatOpenAIBindFailed {
        addr: SocketAddr,
        #[source]
        source: io::Error,
    },
    #[error("Unable to bind the TypeSafe compatibility service to {addr}")]
    CompatTypeSafeBindFailed {
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
    #[error(transparent)]
    ServiceThread(#[from] ServiceThreadError),
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
