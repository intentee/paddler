use std::io;
use std::net::AddrParseError;
use std::net::SocketAddr;
use std::num::ParseIntError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormFieldError {
    #[error("Cannot bind to {addr}: {source}")]
    AddressCannotBeBound {
        addr: SocketAddr,
        #[source]
        source: io::Error,
    },
    #[error("Port {port} is already in use")]
    AddressPortInUse { port: u16 },
    #[error("Address is required.")]
    AddressRequired,
    #[error("Invalid address ({source}), expected format: IP:port")]
    AddressUnparsable {
        #[source]
        source: AddrParseError,
    },
    #[error("Cluster address is required.")]
    ClusterAddressRequired,
    #[error("Please select a model.")]
    ModelNotSelected,
    #[error("Invalid number of slots.")]
    SlotsInvalid {
        #[source]
        source: ParseIntError,
    },
    #[error("Invalid number of slots (the number should be greater than zero).")]
    SlotsNotPositive,
    #[error("Number of slots is required.")]
    SlotsRequired,
    #[error("Number of slots cannot exceed {limit}.")]
    SlotsAboveLimit { limit: u16 },
}
