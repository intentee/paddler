use std::io;
use std::net::SocketAddr;
use std::net::TcpListener;

pub enum PortCheck {
    Available,
    BindFailed(io::Error),
    InUse,
}

impl PortCheck {
    #[must_use]
    pub fn of(address: &SocketAddr) -> Self {
        match TcpListener::bind(address) {
            Ok(_listener) => Self::Available,
            Err(error) if error.kind() == io::ErrorKind::AddrInUse => Self::InUse,
            Err(error) => Self::BindFailed(error),
        }
    }
}
