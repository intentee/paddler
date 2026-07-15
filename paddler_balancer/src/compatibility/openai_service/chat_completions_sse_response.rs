use std::convert::Infallible;
use std::fmt::Debug;
use std::sync::Arc;

use actix_web::HttpResponse;
use actix_web::http::header;
use actix_web_lab::sse;
use futures::stream::StreamExt as _;
use futures::stream::once;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::streamable_result::StreamableResult;
use tokio_util::sync::CancellationToken;

use crate::agent_controller::AgentController;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::manages_senders::ManagesSenders;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;

pub fn chat_completions_sse_response<TParams, TTransformsOutgoingMessage>(
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
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage<Output = TransformResult> + Send + Sync + 'static,
{
    let event_stream = unbounded_stream_from_agent(
        buffered_request_manager,
        inference_service_configuration,
        params,
        transformer,
        shutdown,
    )
    .filter_map(|transform_result| async move {
        match transform_result {
            TransformResult::Chunk(chunk) | TransformResult::Error(chunk) => {
                Some(Ok::<sse::Event, Infallible>(sse::Event::Data(
                    sse::Data::new(chunk),
                )))
            }
            TransformResult::Discard => None,
        }
    })
    .chain(once(async {
        Ok::<sse::Event, Infallible>(sse::Event::Data(sse::Data::new("[DONE]")))
    }));

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body(sse::Sse::from_stream(event_stream))
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use actix_web::body;
    use tokio_util::sync::CancellationToken;

    use super::chat_completions_sse_response;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
    use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

    fn empty_pool_manager() -> Arc<BufferedRequestManager> {
        Arc::new(BufferedRequestManager::new(
            Arc::new(AgentControllerPool::default()),
            Duration::from_secs(1),
            10,
        ))
    }

    fn inference_service_configuration() -> InferenceServiceConfiguration {
        InferenceServiceConfiguration {
            addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            cors_allowed_hosts: Vec::new(),
            inference_item_timeout: Duration::from_secs(1),
        }
    }

    fn raw_prompt_params() -> ContinueFromRawPromptParams {
        ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 1,
            raw_prompt: "hello".to_owned(),
        }
    }

    #[actix_web::test]
    async fn frames_each_chunk_as_an_sse_data_event_and_terminates_with_done() {
        let shutdown = CancellationToken::new();
        shutdown.cancel();

        let response = chat_completions_sse_response(
            empty_pool_manager(),
            inference_service_configuration(),
            raw_prompt_params(),
            IdentityTransformer::new(),
            shutdown,
        );

        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/event-stream"
        );

        let body_bytes = body::to_bytes(response.into_body()).await.unwrap();
        let body_text = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(
            body_text.contains("data: "),
            "each chunk must be framed as an SSE data event: {body_text}"
        );
        assert!(
            body_text.contains("balancer is shutting down"),
            "the shutdown chunk must be framed into the SSE body: {body_text}"
        );
        assert!(
            body_text.ends_with("data: [DONE]\n\n"),
            "the stream must terminate with the OpenAI [DONE] sentinel: {body_text:?}"
        );
    }
}
