use std::net::SocketAddr;
use std::num::NonZeroU32;

use futures_util::SinkExt as _;
use futures_util::StreamExt as _;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
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

fn awaited_answer(
    frame: Message,
    is_awaited: impl Fn(&str) -> bool,
) -> Option<Result<InferenceClientMessage, ClusterHarnessError>> {
    match frame {
        Message::Text(text) => from_str::<InferenceClientMessage>(&text)
            .map_err(ClusterHarnessError::InferenceSocketMessageUnreadable)
            .map(|message| is_awaited(answered_request_id(&message)).then_some(message))
            .transpose(),
        _other_frame => None,
    }
}

fn pong_payload(frame: Message) -> Option<Result<Bytes, ClusterHarnessError>> {
    match frame {
        Message::Pong(payload) => Some(Ok(payload)),
        _other_frame => None,
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
        self.next_selected_frame(|frame| awaited_answer(frame, |_answered_request_id| true))
            .await
            .and_then(|answer| {
                answer.ok_or(ClusterHarnessError::InferenceSocketClosedBeforeAnyAnswer)
            })
    }

    pub async fn next_answer_to(
        &mut self,
        request_id: &str,
    ) -> Result<InferenceClientMessage, ClusterHarnessError> {
        self.next_selected_frame(|frame| {
            awaited_answer(frame, |answered_request_id| {
                answered_request_id == request_id
            })
        })
        .await
        .and_then(|answer| {
            answer.ok_or_else(|| ClusterHarnessError::InferenceSocketClosedBeforeAnswer {
                request_id: request_id.to_owned(),
            })
        })
    }

    pub async fn next_pong(&mut self) -> Result<Bytes, ClusterHarnessError> {
        self.next_selected_frame(pong_payload)
            .await
            .and_then(|pong| pong.ok_or(ClusterHarnessError::InferenceSocketClosedBeforePong))
    }

    async fn next_selected_frame<TSelected>(
        &mut self,
        select: impl Fn(Message) -> Option<Result<TSelected, ClusterHarnessError>>,
    ) -> Result<Option<TSelected>, ClusterHarnessError> {
        while let Some(frame) = self.next_frame_before_close().await? {
            if let Some(selected) = select(frame) {
                return selected.map(Some);
            }
        }

        Ok(None)
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

        ready(
            to_string(&request)
                .map(Message::text)
                .map_err(ClusterHarnessError::InferenceSocketRequestUnserializable),
        )
        .and_then(|serialized_request| {
            self.send(serialized_request)
                .map_err(ClusterHarnessError::InferenceSocketSendFailed)
        })
        .await
    }
}
