use std::sync::Arc;

use futures_util::StreamExt as _;
use log::debug;
use log::error;
use log::warn;
use serde_json::from_str;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::protocol::Message;

use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::management_socket::agent::message::Message as JsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as JsonRpcNotification;
use paddler_messaging::management_socket::agent::notification_params::version_params::VersionParams;
use paddler_messaging::management_socket::agent::request::Request as JsonRpcRequest;
use paddler_messaging::management_socket::agent::response::Response as JsonRpcResponse;
use paddler_messaging::reserved_request::ReservedRequest;
use paddler_outbox::outbox::Outbox;
use paddler_request_registry::request_delivery::RequestDelivery;
use paddler_request_registry::request_registry::RequestRegistry;
use paddler_request_registry::request_registry_guard::RequestRegistryGuard;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::agent_response_message::agent_response_message;
use crate::management_connection_end::ManagementConnectionEnd;
use crate::management_connection_step::ManagementConnectionStep;
use crate::pipeline_request::PipelineRequest;
use crate::pipeline_response_stream::PipelineResponseStream;

pub struct BalancerMessageContext {
    pub agent_desired_state_tx: watch::Sender<AgentDesiredState>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub pipeline_requests: Arc<Outbox<PipelineRequest>>,
    pub request_stoppers: Arc<RequestRegistry<mpsc::UnboundedSender<()>>>,
    pub served_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl BalancerMessageContext {
    #[must_use]
    pub fn handle_message(&self, message: Message) -> ManagementConnectionStep {
        match message {
            Message::Text(text) => match from_str::<JsonRpcMessage>(&text) {
                Ok(deserialized_message) => self.handle_deserialized_message(deserialized_message),
                Err(deserialization_error) => {
                    error!(
                        "Closing the management connection after an undeserializable message {text}: {deserialization_error}"
                    );

                    ManagementConnectionStep::End(
                        ManagementConnectionEnd::BalancerMessageUndeserializable,
                    )
                }
            },
            Message::Binary(_) | Message::Frame(_) => {
                error!("Closing the management connection after a non-text message");

                ManagementConnectionStep::End(ManagementConnectionEnd::BalancerSentUnsupportedFrame)
            }
            Message::Close(_) | Message::Ping(_) | Message::Pong(_) => {
                ManagementConnectionStep::Continue
            }
        }
    }

    fn handle_deserialized_message(
        &self,
        deserialized_message: JsonRpcMessage,
    ) -> ManagementConnectionStep {
        match deserialized_message {
            JsonRpcMessage::Notification(JsonRpcNotification::SetState(set_state_params)) => {
                let desired_state = set_state_params.desired_state;

                self.agent_desired_state_tx
                    .send_if_modified(|current_desired_state| {
                        if *current_desired_state == desired_state {
                            return false;
                        }

                        *current_desired_state = desired_state;

                        true
                    });

                ManagementConnectionStep::Continue
            }
            JsonRpcMessage::Notification(JsonRpcNotification::StopRespondingTo(request_id)) => {
                match self.request_stoppers.send_to(&request_id, ()) {
                    RequestDelivery::Delivered => {}
                    RequestDelivery::ReceiverDropped | RequestDelivery::RequestNotRegistered => {
                        debug!("Request {request_id:?} finished before its stop arrived");
                    }
                }

                ManagementConnectionStep::Continue
            }
            JsonRpcMessage::Notification(JsonRpcNotification::Version(VersionParams {
                version,
            })) => {
                if version != env!("CARGO_PKG_VERSION") {
                    warn!(
                        "Version mismatch: server version is {version}, client version is {}",
                        env!("CARGO_PKG_VERSION")
                    );
                }

                ManagementConnectionStep::Continue
            }
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request:
                    JsonRpcRequest::ContinueFromConversationHistory(
                        continue_from_conversation_history_params,
                    ),
            }) => self.stream_responses::<_, GeneratedTokenResult>(
                id,
                continue_from_conversation_history_params,
            ),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::ContinueFromRawPrompt(continue_from_raw_prompt_params),
            }) => self
                .stream_responses::<_, GeneratedTokenResult>(id, continue_from_raw_prompt_params),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::Decide(decide_params),
            }) => self.stream_responses::<_, DecisionResult>(id, decide_params),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GenerateEmbeddingBatch(generate_embedding_batch_params),
            }) => self.stream_responses::<_, EmbeddingResult>(id, generate_embedding_batch_params),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GetChatTemplateOverride,
            }) => ManagementConnectionStep::Write(agent_response_message(
                id,
                JsonRpcResponse::ChatTemplateOverride(
                    self.served_applicable_state_holder
                        .get_agent_applicable_state()
                        .chat_template_override(),
                ),
            )),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GetModelMetadata,
            }) => ManagementConnectionStep::Write(agent_response_message(
                id,
                JsonRpcResponse::ModelMetadata(self.model_metadata_holder.get_model_metadata()),
            )),
        }
    }

    fn stream_responses<TParams, TResponse>(
        &self,
        request_id: String,
        ReservedRequest {
            params,
            reserved_slots,
        }: ReservedRequest<TParams>,
    ) -> ManagementConnectionStep
    where
        AgentRequest<TParams, TResponse>: Into<PipelineRequest>,
        TResponse: Into<JsonRpcResponse> + Send + 'static,
    {
        let (response_tx, response_rx) = mpsc::unbounded_channel::<TResponse>();
        let (stop_tx, stop_rx) = mpsc::unbounded_channel::<()>();

        let Some(stopper_guard) =
            RequestRegistryGuard::register(&self.request_stoppers, request_id.clone(), stop_tx)
        else {
            error!(
                "The balancer reused the id of request {request_id:?}, which is still in flight"
            );

            return ManagementConnectionStep::End(
                ManagementConnectionEnd::BalancerReusedInFlightRequestId,
            );
        };

        self.pipeline_requests.push(
            AgentRequest {
                params,
                response_tx,
                slot_guard: SlotGuard::new(self.slot_aggregated_status.clone(), reserved_slots),
                stop_rx,
            }
            .into(),
        );

        ManagementConnectionStep::StreamResponses(
            PipelineResponseStream {
                response_rx,
                stopper_guard,
            }
            .boxed(),
        )
    }
}
