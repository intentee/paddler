use std::convert::Infallible;
use std::fmt::Debug;
use std::sync::Arc;

use actix_web::HttpResponse;
use actix_web::http::header;
use actix_web_lab::sse;
use futures::stream::StreamExt as _;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::streamable_result::StreamableResult;
use tokio_util::sync::CancellationToken;

use crate::agent_controller::AgentController;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::compatibility::openai_service::responses_stream_event::ResponsesStreamEvent;
use crate::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::manages_senders::ManagesSenders;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;

fn event_to_sse_data(event: &ResponsesStreamEvent) -> sse::Data {
    sse::Data::new(event.to_json().to_string()).event(event.event_name())
}

pub fn sse_response_from_agent<TParams, TTransformsOutgoingMessage>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    inference_service_configuration: InferenceServiceConfiguration,
    params: TParams,
    transformer: TTransformsOutgoingMessage,
    shutdown: CancellationToken,
) -> HttpResponse
where
    TParams: Debug + Into<AgentJsonRpcRequest> + Send + 'static,
    AgentController: HandlesAgentStreamingResponse<TParams>,
    <<AgentController as HandlesAgentStreamingResponse<TParams>>::SenderCollection as ManagesSenders>::Value: Debug + Into<OutgoingResponse> + StreamableResult,
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage<Output = ResponsesStreamEvent> + Send + Sync + 'static,
{
    let event_stream = unbounded_stream_from_agent(
        buffered_request_manager,
        inference_service_configuration,
        params,
        transformer,
        shutdown,
    )
    .map(|event| Ok::<sse::Event, Infallible>(sse::Event::Data(event_to_sse_data(&event))));

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body(sse::Sse::from_stream(event_stream))
}
