use std::io;
use std::net::SocketAddr;

use serde_json::Error as SerdeJsonError;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::watch::error::RecvError;

use paddler_state_database::state_database_error::StateDatabaseError;

use crate::transform_result::TransformResult;
use crate::websocket_outbound_frame::WebSocketOutboundFrame;

#[derive(Debug, Error)]
pub enum BalancerError {
    #[error("the desired state store stopped announcing applicable states")]
    ApplicableStatesNoLongerAnnounced(#[source] RecvError),
    #[error("the client stopped receiving the response chunks")]
    ChunkReceiverClosed(#[source] SendError<TransformResult>),
    #[error("Unable to listen for {service_name} on {local_addr}")]
    HttpServiceCannotListen {
        local_addr: SocketAddr,
        service_name: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("Unable to serve {service_name} on {local_addr}")]
    HttpServiceFailed {
        local_addr: SocketAddr,
        service_name: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("unable to serialize a response for the WebSocket session")]
    ResponseUnserializable(#[source] SerdeJsonError),
    #[error(transparent)]
    StateDatabase(#[from] StateDatabaseError),
    #[error("unable to bind the socket that reports metrics to statsd")]
    StatsdSocketUnbindable(#[source] io::Error),
    #[error(
        "The balancer desired state at version {version} cannot be replaced, because no later version is representable"
    )]
    VersionsExhausted { version: u64 },
    #[error("the WebSocket session stopped accepting outbound frames")]
    WebSocketOutboundClosed(#[source] SendError<WebSocketOutboundFrame>),
}
