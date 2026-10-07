use std::net::SocketAddr;

use futures_util::SinkExt as _;
use futures_util::StreamExt as _;
use serde_json::from_str;
use serde_json::to_string;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::cluster_harness_error::ClusterHarnessError;

pub struct RawAgentSocket {
    websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl RawAgentSocket {
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

    async fn next_message(&mut self) -> Result<AgentJsonRpcMessage, ClusterHarnessError> {
        while let Some(frame) = self.websocket.next().await {
            if let Message::Text(text) =
                frame.map_err(ClusterHarnessError::AgentSocketReceiveFailed)?
            {
                return from_str(&text)
                    .map_err(ClusterHarnessError::AgentSocketMessageUndeserializable);
            }
        }

        Err(ClusterHarnessError::AgentSocketEndedWithoutMessage)
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

    pub async fn next_request(
        &mut self,
    ) -> Result<RequestEnvelope<AgentJsonRpcRequest>, ClusterHarnessError> {
        loop {
            if let AgentJsonRpcMessage::Request(request_envelope) = self.next_message().await? {
                return Ok(request_envelope);
            }
        }
    }

    pub async fn register(&mut self) -> Result<(), ClusterHarnessError> {
        self.register_with_status(SlotAggregatedStatusSnapshot::default())
            .await
    }

    pub async fn register_with_status(
        &mut self,
        slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot,
    ) -> Result<(), ClusterHarnessError> {
        self.send_notification(ManagementJsonRpcNotification::RegisterAgent(
            RegisterAgentParams {
                name: None,
                slot_aggregated_status_snapshot,
            },
        ))
        .await?;

        loop {
            if let AgentJsonRpcMessage::Notification(AgentJsonRpcNotification::SetState(_)) =
                self.next_message().await?
            {
                return Ok(());
            }
        }
    }

    pub async fn send(&mut self, message: Message) -> Result<(), ClusterHarnessError> {
        self.websocket
            .send(message)
            .await
            .map_err(ClusterHarnessError::AgentSocketSendFailed)
    }

    pub async fn send_notification(
        &mut self,
        notification: ManagementJsonRpcNotification,
    ) -> Result<(), ClusterHarnessError> {
        let serialized_notification =
            to_string(&ManagementJsonRpcMessage::Notification(notification))
                .map_err(ClusterHarnessError::AgentSocketNotificationUnserializable)?;

        self.send(Message::text(serialized_notification)).await
    }

    pub async fn send_response(
        &mut self,
        response_envelope: ResponseEnvelope<AgentJsonRpcResponse>,
    ) -> Result<(), ClusterHarnessError> {
        let serialized_response = to_string(&ManagementJsonRpcMessage::Response(response_envelope))
            .map_err(ClusterHarnessError::AgentSocketResponseUnserializable)?;

        self.send(Message::text(serialized_response)).await
    }
}
