use std::net::SocketAddr;
use std::num::NonZeroU32;

use futures_util::SinkExt as _;
use futures_util::StreamExt as _;
use serde_json::from_str;
use serde_json::to_string;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::Message;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

use crate::cluster_harness_error::ClusterHarnessError;

const fn answered_request_id(message: &InferenceClientMessage) -> &str {
    match message {
        InferenceClientMessage::Error(ErrorEnvelope { request_id, .. })
        | InferenceClientMessage::Response(ResponseEnvelope { request_id, .. }) => {
            request_id.as_str()
        }
    }
}

pub struct RawInferenceSocket {
    websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl RawInferenceSocket {
    pub async fn connect(inference_addr: SocketAddr) -> Result<Self, WebSocketError> {
        connect_async(format!(
            "ws://{inference_addr}{}",
            ApiPath::INFERENCE_SOCKET
        ))
        .await
        .map(|(websocket, _handshake_response)| Self { websocket })
    }

    pub async fn next_answer(&mut self) -> Result<InferenceClientMessage, ClusterHarnessError> {
        self.next_message()
            .await?
            .ok_or(ClusterHarnessError::InferenceSocketClosedBeforeAnyAnswer)
    }

    pub async fn next_answer_to(
        &mut self,
        request_id: &str,
    ) -> Result<InferenceClientMessage, ClusterHarnessError> {
        while let Some(message) = self.next_message().await? {
            if answered_request_id(&message) == request_id {
                return Ok(message);
            }
        }

        Err(ClusterHarnessError::InferenceSocketClosedBeforeAnswer {
            request_id: request_id.to_owned(),
        })
    }

    async fn next_message(
        &mut self,
    ) -> Result<Option<InferenceClientMessage>, ClusterHarnessError> {
        while let Some(frame) = self.next_frame_before_close().await? {
            if let Message::Text(text) = frame {
                return from_str::<InferenceClientMessage>(&text)
                    .map(Some)
                    .map_err(ClusterHarnessError::InferenceSocketMessageUnreadable);
            }
        }

        Ok(None)
    }

    pub async fn next_pong(&mut self) -> Result<Bytes, ClusterHarnessError> {
        while let Some(frame) = self.next_frame_before_close().await? {
            if let Message::Pong(payload) = frame {
                return Ok(payload);
            }
        }

        Err(ClusterHarnessError::InferenceSocketClosedBeforePong)
    }

    async fn next_frame_before_close(&mut self) -> Result<Option<Message>, ClusterHarnessError> {
        match self.websocket.next().await {
            None | Some(Ok(Message::Close(_))) => Ok(None),
            Some(frame) => frame
                .map(Some)
                .map_err(ClusterHarnessError::InferenceSocketReceiveFailed),
        }
    }

    pub async fn send(&mut self, message: Message) -> Result<(), WebSocketError> {
        self.websocket.send(message).await
    }

    pub async fn send_raw_prompt_request(
        &mut self,
        request_id: &str,
        max_tokens: NonZeroU32,
    ) -> Result<(), ClusterHarnessError> {
        let request: InferenceServerMessage<ValidatedParametersSchema> =
            InferenceServerMessage::Request(RequestEnvelope {
                id: request_id.to_owned(),
                request: InferenceServerRequest::ContinueFromRawPrompt(
                    ContinueFromRawPromptParams {
                        grammar: None,
                        max_tokens,
                        raw_prompt: "Hello".to_owned(),
                    },
                ),
            });

        let serialized_request = to_string(&request)
            .map_err(ClusterHarnessError::InferenceSocketRequestUnserializable)?;

        self.send(Message::text(serialized_request))
            .await
            .map_err(ClusterHarnessError::InferenceSocketSendFailed)
    }
}
