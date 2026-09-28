use std::net::SocketAddr;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClusterHarnessError {
    #[error("Unable to build a base URL for {addr}")]
    BaseUrlInvalid {
        addr: SocketAddr,
        #[source]
        source: url::ParseError,
    },
    #[error("The balancer does not serve the OpenAI compatibility service")]
    CompatOpenAIServiceNotServed,
    #[error("The half-closed client could not reach {addr}")]
    HalfClosedClientUnreachable {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
}
