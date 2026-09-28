use std::net::SocketAddr;

use anyhow::Result;
use anyhow::bail;
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
use tokio_tungstenite::tungstenite::Message;

pub struct RawInferenceSocket {
    websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl RawInferenceSocket {
    pub async fn connect(inference_addr: SocketAddr) -> Result<Self> {
        let (websocket, _handshake_response) =
            connect_async(format!("ws://{inference_addr}/api/v1/inference_socket")).await?;

        Ok(Self { websocket })
    }

    pub async fn next_answer_to(&mut self, request_id: &str) -> Result<InferenceClientMessage> {
        while let Some(frame) = self.websocket.next().await {
            if let Message::Text(text) = frame? {
                let message = serde_json::from_str::<InferenceClientMessage>(&text)?;
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

        bail!("the inference socket closed before answering request {request_id}")
    }

    pub async fn next_pong(&mut self) -> Result<Bytes> {
        while let Some(frame) = self.websocket.next().await {
            if let Message::Pong(payload) = frame? {
                return Ok(payload);
            }
        }

        bail!("the inference socket closed before answering the ping")
    }

    pub async fn send(&mut self, message: Message) -> Result<()> {
        Ok(self.websocket.send(message).await?)
    }

    pub async fn send_raw_prompt_request(&mut self, request_id: &str) -> Result<()> {
        let request: InferenceServerMessage<ValidatedParametersSchema> =
            InferenceServerMessage::Request(RequestEnvelope {
                id: request_id.to_owned(),
                request: InferenceServerRequest::ContinueFromRawPrompt(
                    ContinueFromRawPromptParams {
                        grammar: None,
                        max_tokens: 1,
                        raw_prompt: "Hello".to_owned(),
                    },
                ),
            });

        self.send(Message::text(serde_json::to_string(&request)?))
            .await
    }
}
