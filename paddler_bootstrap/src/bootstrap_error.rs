use std::io;
use std::net::SocketAddr;

use thiserror::Error;
use trzcina::service_shutdown_error::ServiceShutdownError;

use paddler_balancer::balancer_error::BalancerError;
use paddler_error_chain::error_chains::ErrorChains;

#[derive(Debug, Error)]
pub enum BootstrapError {
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
    #[error("Paddler services stopped with errors: {}", ErrorChains { errors: failures })]
    ServiceRunFailed { failures: Vec<ServiceShutdownError> },
    #[error("A Paddler service thread panicked")]
    ServiceThreadPanicked,
    #[error("The statsd reporting interval must be longer than zero")]
    StatsdReportingIntervalIsZero,
    #[error("Unable to read the balancer desired state from the state database")]
    StateDatabaseReadFailed {
        #[source]
        source: BalancerError,
    },
    #[cfg(feature = "web_admin_panel")]
    #[error("Unable to bind the web admin panel service to {addr}")]
    WebAdminPanelBindFailed {
        addr: SocketAddr,
        #[source]
        source: io::Error,
    },
}
