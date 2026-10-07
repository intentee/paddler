use std::fmt::Debug;

use actix_web::rt;
use futures_util::Stream;
use nanoid::nanoid;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_streaming_request::AgentStreamingRequest;
use crate::cancellation_token_stream_guard::CancellationTokenStreamGuard;
use crate::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::request_from_agent::request_from_agent;
use crate::unbounded_stream_from_agent_params::UnboundedStreamFromAgentParams;

pub fn unbounded_stream_from_agent<TParams, TTransformsOutgoingMessage>(
    UnboundedStreamFromAgentParams {
        buffered_request_manager,
        inference_service_configuration,
        request_params,
        shutdown,
        transformer,
    }: UnboundedStreamFromAgentParams<TParams, TTransformsOutgoingMessage>,
) -> impl Stream<Item = TTransformsOutgoingMessage::Output>
where
    TParams: AgentStreamingRequest + Debug + Send + 'static,
    TParams::Response: Debug + Into<OutgoingResponse> + StreamableResult,
    TTransformsOutgoingMessage: TransformsOutgoingMessage + Send + Sync + 'static,
{
    let request_id: String = nanoid!();
    let connection_close = CancellationToken::new();
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

    rt::spawn({
        let connection_close = connection_close.clone();

        async move {
            let session_controller = ChunkForwardingSessionController::new(chunk_tx, transformer);

            request_from_agent(
                buffered_request_manager,
                connection_close,
                inference_service_configuration,
                request_params,
                request_id,
                session_controller,
                shutdown,
            )
            .await;
        }
    });

    CancellationTokenStreamGuard::new(connection_close, UnboundedReceiverStream::new(chunk_rx))
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use std::time::Duration;

    use futures_util::StreamExt as _;
    use serde_json::from_str;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

    use super::unbounded_stream_from_agent;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
    use crate::resolved_socket_addr::ResolvedSocketAddr;
    use crate::unbounded_stream_from_agent_params::UnboundedStreamFromAgentParams;

    fn inference_service_configuration() -> InferenceServiceConfiguration {
        const TIMEOUT_LONGER_THAN_ANY_TEST_RUN: Duration = Duration::from_hours(1);

        InferenceServiceConfiguration {
            addr: ResolvedSocketAddr::from(SocketAddr::from(([127, 0, 0, 1], 0))),
            cors_allowed_hosts: Vec::new(),
            inference_item_timeout: TIMEOUT_LONGER_THAN_ANY_TEST_RUN,
        }
    }

    #[actix_web::test]
    async fn spawned_task_runs_request_from_agent_and_closes_stream_on_shutdown() {
        let pool = Arc::new(AgentControllerPool::new(InferenceMode::TextGeneration));
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_secs(1),
            10,
        ));

        let shutdown = CancellationToken::new();

        shutdown.cancel();

        let mut stream = Box::pin(unbounded_stream_from_agent(
            UnboundedStreamFromAgentParams {
                buffered_request_manager,
                inference_service_configuration: inference_service_configuration(),
                request_params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::new(1).unwrap(),
                    raw_prompt: "fixture prompt".to_owned(),
                },
                shutdown,
                transformer: IdentityTransformer::new(),
            },
        ));

        let Some(TransformResult::Chunk(shutdown_chunk)) = stream.next().await else {
            panic!("the stream must forward the shutdown as a chunk");
        };

        assert!(matches!(
            from_str::<OutgoingMessage>(&shutdown_chunk).unwrap(),
            OutgoingMessage::Error(ErrorEnvelope {
                error: JsonRpcError {
                    code: 503,
                    description,
                },
                ..
            }) if description == "balancer is shutting down"
        ));
        assert!(stream.next().await.is_none());
    }
}
