use std::io;

use thiserror::Error;
use tokio::sync::watch::error::RecvError;

use paddler_tcp_buffer_limits::tcp_buffer_limits_error::TcpBufferLimitsError;

#[derive(Debug, Error)]
pub enum ResettingClientError {
    #[error("The resetting client cannot connect to the fixture")]
    FixtureUnreachable(#[source] io::Error),
    #[error(
        "No body outlasts both the largest send buffer of {largest_send_buffer} bytes and the receive buffer of {receive_buffer} bytes"
    )]
    OutlastingBodyLengthOverflows {
        largest_send_buffer: usize,
        receive_buffer: usize,
    },
    #[error("The resetting client cannot read back its receive buffer size")]
    ReceiveBufferUnreadable(#[source] io::Error),
    #[error("The resetting client cannot shrink its receive buffer")]
    ReceiveBufferUnshrinkable(#[source] io::Error),
    #[error(
        "The fixture stopped counting requests before it read the request of the resetting client"
    )]
    RequestUnobserved(#[source] RecvError),
    #[error("The resetting client cannot send its request")]
    RequestUnsent(#[source] io::Error),
    #[error("The resetting client cannot read the TCP send buffer limits")]
    SendBufferLimitsUnreadable(#[source] TcpBufferLimitsError),
    #[error("The resetting client cannot open its socket")]
    SocketUnopenable(#[source] io::Error),
    #[error("The resetting client cannot make closing its connection reset it")]
    ZeroLingerUnset(#[source] io::Error),
}
