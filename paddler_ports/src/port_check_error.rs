use std::io;
use std::net::AddrParseError;
use std::net::SocketAddr;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PortCheckError {
    #[error("Address is required.")]
    Empty,
    #[error("Invalid address ({source}), expected format: IP:port")]
    Unparseable {
        #[from]
        source: AddrParseError,
    },
    #[error("Port {} is already in use", socket_addr.port())]
    InUse { socket_addr: SocketAddr },
    #[error("Cannot bind to {socket_addr}: {source}")]
    BindFailed {
        socket_addr: SocketAddr,
        source: io::Error,
    },
}

impl PortCheckError {
    pub fn user_facing_message(&self) -> String {
        self.to_string()
    }
}
