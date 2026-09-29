use std::net::SocketAddr;
use std::num::NonZeroU32;

use futures_util::SinkExt as _;
use futures_util::StreamExt as _;
use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::Message;

use crate::cluster_harness_error::ClusterHarnessError;

pub struct RawInferenceSocket {
    websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl RawInferenceSocket {
    pub async fn connect(inference_addr: SocketAddr) -> Result<Self, WebSocketError> {
        connect_async(format!("ws://{inference_addr}/api/v1/inference_socket"))
            .await
            .map(|(websocket, _handshake_response)| Self { websocket })
    }

    pub async fn next_answer_to(
        &mut self,
        request_id: &str,
    ) -> Result<InferenceClientMessage, ClusterHarnessError> {
        while let Some(frame) = self.next_frame_before_close().await? {
            if let Message::Text(text) = frame {
                let message = serde_json::from_str::<InferenceClientMessage>(&text)
                    .map_err(ClusterHarnessError::InferenceSocketMessageUnreadable)?;
                let answered_request_id = match &message {
                    InferenceClientMessage::Error(ErrorEnvelope {
                        request_id: answered_request_id,
                        ..
                    })
                    | InferenceClientMessage::Response(ResponseEnvelope {
                        request_id: answered_request_id,
                        ..
                    }) => Some(answered_request_id.as_str()),
                    InferenceClientMessage::Notification(_) => None,
                };

                if answered_request_id == Some(request_id) {
                    return Ok(message);
                }
            }
        }

        Err(ClusterHarnessError::InferenceSocketClosedBeforeAnswer {
            request_id: request_id.to_owned(),
        })
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

        let serialized_request = serde_json::to_string(&request)
            .map_err(ClusterHarnessError::InferenceSocketRequestUnserializable)?;

        self.send(Message::text(serialized_request))
            .await
            .map_err(ClusterHarnessError::InferenceSocketSendFailed)
    }
}
