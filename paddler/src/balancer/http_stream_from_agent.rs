use std::fmt::Debug;
use std::sync::Arc;

use crate::balancer::inference_client::Response as OutgoingResponse;
use crate::streamable_result::StreamableResult;
use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::http::header;
use bytes::Bytes;
use futures::stream::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::agent::jsonrpc::Request as AgentJsonRpcRequest;
use crate::balancer::agent_controller::AgentController;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::balancer::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::balancer::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::balancer::manages_senders::ManagesSenders;
use crate::balancer::unbounded_stream_from_agent::unbounded_stream_from_agent;

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
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage + Send + Sync + 'static,
{
    let stream = unbounded_stream_from_agent(
        buffered_request_manager,
        inference_service_configuration,
        params,
        transformer,
        shutdown,
    )
    .filter_map(|transform_result| async move {
        match transform_result {
            TransformResult::Chunk(chunk) => {
                Some(Ok::<_, Error>(Bytes::from(format!("{chunk}\n"))))
            }
            TransformResult::Error(error) => {
                Some(Ok::<_, Error>(Bytes::from(format!("{error}\n"))))
            }
            TransformResult::Discard => None,
        }
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
    use crate::balancer::agent_controller_pool::AgentControllerPool;
    use crate::balancer::buffered_request_manager::BufferedRequestManager;
    use crate::balancer::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
    use crate::balancer::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::balancer::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
    use crate::balancer::inference_client::Message as OutgoingMessage;
    use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
    use crate::request_params::ContinueFromRawPromptParams;

    #[derive(Clone)]
    struct ErrorTransformer;

    #[async_trait]
    impl TransformsOutgoingMessage for ErrorTransformer {
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
