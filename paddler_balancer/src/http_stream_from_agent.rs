use std::fmt::Debug;
use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::http::header;
use futures::stream::StreamExt;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::streamable_result::StreamableResult;
use tokio_util::sync::CancellationToken;

use crate::agent_controller::AgentController;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::manages_senders::ManagesSenders;
use crate::sse_line_bytes::sse_line_bytes;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;

pub fn http_stream_from_agent<TParams, TTransformsOutgoingMessage>(
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
    let stream = unbounded_stream_from_agent(
        buffered_request_manager,
        inference_service_configuration,
        params,
        transformer,
        shutdown,
    )
    .filter_map(|transform_result| async move {
        sse_line_bytes(transform_result).map(Ok::<_, Error>)
    });

    HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .streaming(stream)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use actix_web::body;
    use anyhow::Result;
    use async_trait::async_trait;
    use tokio_util::sync::CancellationToken;

    use super::http_stream_from_agent;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
    use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

    #[derive(Clone)]
    struct ErrorTransformer;

    #[async_trait]
    impl TransformsOutgoingMessage for ErrorTransformer {
        type Output = TransformResult;

        async fn transform(&self, _message: OutgoingMessage) -> Result<Vec<TransformResult>> {
            Ok(vec![TransformResult::Error("boom".to_owned())])
        }
    }

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
    async fn forwards_transformed_chunk_to_streaming_body() {
        let shutdown = CancellationToken::new();
        shutdown.cancel();

        let response = http_stream_from_agent(
            empty_pool_manager(),
            inference_service_configuration(),
            raw_prompt_params(),
            IdentityTransformer::new(),
            shutdown,
        );

        let body_bytes = body::to_bytes(response.into_body()).await.unwrap();
        let body_text = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(
            body_text.contains("balancer is shutting down"),
            "chunk arm must serialize the shutdown error envelope into the streaming body: {body_text}"
        );
        assert!(
            body_text.ends_with('\n'),
            "chunk arm must append a trailing newline: {body_text:?}"
        );
    }

    #[actix_web::test]
    async fn forwards_transformed_error_to_streaming_body() {
        let shutdown = CancellationToken::new();
        shutdown.cancel();

        let response = http_stream_from_agent(
            empty_pool_manager(),
            inference_service_configuration(),
            raw_prompt_params(),
            ErrorTransformer,
            shutdown,
        );

        let body_bytes = body::to_bytes(response.into_body()).await.unwrap();
        let body_text = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert_eq!(
            body_text, "boom\n",
            "error arm must forward the transformer error string with a trailing newline"
        );
    }
}
