use std::sync::Arc;

use futures_util::StreamExt as _;
use log::debug;
use log::error;
use log::warn;
use serde_json::from_str;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
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
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub agent_desired_state_tx: mpsc::UnboundedSender<AgentDesiredState>,
    pub pipeline_request_tx: mpsc::UnboundedSender<PipelineRequest>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub request_stoppers: Arc<RequestRegistry<mpsc::UnboundedSender<()>>>,
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
                match self
                    .agent_desired_state_tx
                    .send(set_state_params.desired_state)
                {
                    Ok(()) => ManagementConnectionStep::Continue,
                    Err(SendError(_undelivered_desired_state)) => {
                        error!("The desired state reconciler stopped; deregistering the agent");

                        ManagementConnectionStep::Deregister
                    }
                }
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
                    self.agent_applicable_state_holder
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
        params: TParams,
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

        match self.pipeline_request_tx.send(
            AgentRequest {
                params,
                response_tx,
                slot_guard: SlotGuard::new(self.slot_aggregated_status.clone()),
                stop_rx,
            }
            .into(),
        ) {
            Ok(()) => ManagementConnectionStep::StreamResponses(
                PipelineResponseStream {
                    response_rx,
                    stopper_guard,
                }
                .boxed(),
            ),
            Err(SendError(_undelivered_request)) => {
                error!("The inference arbiter stopped; deregistering the agent");

                ManagementConnectionStep::Deregister
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use tokio::sync::mpsc;
    use tokio_tungstenite::tungstenite::Bytes;
    use tokio_tungstenite::tungstenite::protocol::Message;

    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
    use paddler_messaging::management_socket::agent::message::Message as JsonRpcMessage;
    use paddler_messaging::management_socket::agent::notification::Notification as JsonRpcNotification;
    use paddler_messaging::management_socket::agent::notification_params::set_state_params::SetStateParams;
    use paddler_messaging::management_socket::agent::notification_params::version_params::VersionParams;
    use paddler_messaging::management_socket::agent::request::Request as JsonRpcRequest;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_request_registry::request_delivery::RequestDelivery;
    use paddler_request_registry::request_registry_guard::RequestRegistryGuard;

    use crate::balancer_message_context_fixture::BalancerMessageContextFixture;
    use crate::management_connection_end::ManagementConnectionEnd;
    use crate::management_connection_step::ManagementConnectionStep;
    use crate::pipeline_request::PipelineRequest;

    fn raw_prompt_request(request_id: &str) -> JsonRpcMessage {
        JsonRpcMessage::Request(RequestEnvelope {
            id: request_id.to_owned(),
            request: JsonRpcRequest::ContinueFromRawPrompt(ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "hello".to_owned(),
            }),
        })
    }

    #[test]
    fn set_state_for_a_stopped_reconciler_deregisters_the_agent() {
        let BalancerMessageContextFixture {
            agent_desired_state_rx,
            context,
            ..
        } = BalancerMessageContextFixture::default();

        drop(agent_desired_state_rx);

        assert!(matches!(
            context.handle_deserialized_message(JsonRpcMessage::Notification(
                JsonRpcNotification::SetState(Box::new(SetStateParams {
                    desired_state: AgentDesiredState::from(BalancerDesiredState::default()),
                })),
            )),
            ManagementConnectionStep::Deregister
        ));
    }

    #[test]
    fn stop_responding_to_a_finished_request_retains_nothing() {
        let BalancerMessageContextFixture { context, .. } =
            BalancerMessageContextFixture::default();

        assert!(matches!(
            context.handle_deserialized_message(JsonRpcMessage::Notification(
                JsonRpcNotification::StopRespondingTo("already_finished".to_owned()),
            )),
            ManagementConnectionStep::Continue
        ));
        assert!(
            context
                .request_stoppers
                .with_registered("already_finished", |_registered_stopper| ())
                .is_none()
        );
    }

    #[test]
    fn stop_responding_to_a_registered_request_signals_its_stopper() {
        let BalancerMessageContextFixture { context, .. } =
            BalancerMessageContextFixture::default();
        let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<()>();
        let _stopper_guard = RequestRegistryGuard::register(
            &context.request_stoppers,
            "active_request".to_owned(),
            stop_tx,
        )
        .expect("a fresh request id must register");

        assert!(matches!(
            context.handle_deserialized_message(JsonRpcMessage::Notification(
                JsonRpcNotification::StopRespondingTo("active_request".to_owned()),
            )),
            ManagementConnectionStep::Continue
        ));
        assert_eq!(stop_rx.try_recv(), Ok(()));
    }

    #[test]
    fn a_mismatched_version_keeps_the_connection_open() {
        let BalancerMessageContextFixture { context, .. } =
            BalancerMessageContextFixture::default();

        assert!(matches!(
            context.handle_deserialized_message(JsonRpcMessage::Notification(
                JsonRpcNotification::Version(VersionParams {
                    version: "0.0.0-mismatch".to_owned(),
                }),
            )),
            ManagementConnectionStep::Continue
        ));
    }

    #[test]
    fn undeserializable_text_ends_the_connection() {
        let BalancerMessageContextFixture { context, .. } =
            BalancerMessageContextFixture::default();

        assert!(matches!(
            context.handle_message(Message::Text("not json".into())),
            ManagementConnectionStep::End(ManagementConnectionEnd::BalancerMessageUndeserializable)
        ));
    }

    #[test]
    fn a_binary_frame_ends_the_connection() {
        let BalancerMessageContextFixture { context, .. } =
            BalancerMessageContextFixture::default();

        assert!(matches!(
            context.handle_message(Message::Binary(Bytes::from_static(b"\x00\x01"))),
            ManagementConnectionStep::End(ManagementConnectionEnd::BalancerSentUnsupportedFrame)
        ));
    }

    #[test]
    fn streaming_responses_registers_the_stopper_before_it_returns() {
        let BalancerMessageContextFixture {
            context,
            mut pipeline_request_rx,
            ..
        } = BalancerMessageContextFixture::default();

        let step = context.handle_deserialized_message(raw_prompt_request("req_generate"));
        let dispatched_request = pipeline_request_rx
            .try_recv()
            .expect("the request must be dispatched to the arbiter");

        assert!(matches!(step, ManagementConnectionStep::StreamResponses(_)));
        assert!(matches!(
            &dispatched_request,
            PipelineRequest::ContinueFromRawPrompt(request) if request.params.raw_prompt == "hello"
        ));
        assert_eq!(
            context.request_stoppers.send_to("req_generate", ()),
            RequestDelivery::Delivered
        );
    }

    #[test]
    fn a_request_for_a_stopped_arbiter_deregisters_the_agent_and_releases_its_slot() {
        let BalancerMessageContextFixture {
            context,
            pipeline_request_rx,
            ..
        } = BalancerMessageContextFixture::default();

        drop(pipeline_request_rx);

        assert!(matches!(
            context.handle_deserialized_message(raw_prompt_request("req_generate")),
            ManagementConnectionStep::Deregister
        ));
        assert_eq!(
            context.request_stoppers.send_to("req_generate", ()),
            RequestDelivery::RequestNotRegistered
        );
        assert_eq!(context.slot_aggregated_status.slots_processing_count(), 0);
    }

    #[test]
    fn a_reused_in_flight_request_id_ends_the_connection() {
        let BalancerMessageContextFixture { context, .. } =
            BalancerMessageContextFixture::default();
        let (existing_stop_tx, mut existing_stop_rx) = mpsc::unbounded_channel::<()>();
        let _existing_stopper_guard = RequestRegistryGuard::register(
            &context.request_stoppers,
            "req_generate".to_owned(),
            existing_stop_tx,
        )
        .expect("a fresh request id must register");

        assert!(matches!(
            context.handle_deserialized_message(raw_prompt_request("req_generate")),
            ManagementConnectionStep::End(ManagementConnectionEnd::BalancerReusedInFlightRequestId)
        ));
        assert_eq!(
            context.request_stoppers.send_to("req_generate", ()),
            RequestDelivery::Delivered
        );
        assert_eq!(existing_stop_rx.try_recv(), Ok(()));
    }
}
