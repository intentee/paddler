use std::net::SocketAddr;

use futures_util::SinkExt as _;
use futures_util::StreamExt as _;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::Utf8Bytes;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

use paddler_messaging::api_path::ApiPath;

use crate::cluster_harness_error::ClusterHarnessError;

pub struct UntrustedAgentSocketClient {
    websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl UntrustedAgentSocketClient {
    pub async fn connect(
        management_addr: SocketAddr,
        agent_id: &str,
    ) -> Result<Self, WebSocketError> {
        connect_async(format!(
            "ws://{management_addr}{}",
            ApiPath::agent_socket(agent_id)
        ))
        .await
        .map(|(websocket, _handshake_response)| Self { websocket })
    }

    pub async fn next_close_frame(&mut self) -> Result<Option<CloseFrame>, ClusterHarnessError> {
        while let Some(frame) = self.websocket.next().await {
            if let Message::Close(close_frame) =
                frame.map_err(ClusterHarnessError::AgentSocketReceiveFailed)?
            {
                return Ok(close_frame);
            }
        }

        Err(ClusterHarnessError::AgentSocketEndedWithoutClosing)
    }

    pub async fn next_text(&mut self) -> Result<Utf8Bytes, ClusterHarnessError> {
        while let Some(frame) = self.websocket.next().await {
            let message = frame.map_err(ClusterHarnessError::AgentSocketReceiveFailed)?;

            if let Message::Close(close_frame) = message {
                return Err(ClusterHarnessError::AgentSocketClosedBeforeText { close_frame });
            }

            if let Message::Text(text) = message {
                return Ok(text);
            }
        }

        Err(ClusterHarnessError::AgentSocketEndedWithoutText)
    }

    pub async fn send(&mut self, message: Message) -> Result<(), ClusterHarnessError> {
        self.websocket
            .send(message)
            .await
            .map_err(ClusterHarnessError::AgentSocketSendFailed)
    }
}
