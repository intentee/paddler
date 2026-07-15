use std::fmt::Debug;
use std::sync::Arc;

use actix_web::rt;
use futures_util::Stream;
use nanoid::nanoid;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::streamable_result::StreamableResult;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;

use crate::agent_controller::AgentController;
use crate::awaitable_counter::AwaitableCounter;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::guarded_stream::GuardedStream;
use crate::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::manages_senders::ManagesSenders;
use crate::request_from_agent::request_from_agent;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;

pub fn unbounded_stream_from_agent<TParams, TTransformsOutgoingMessage>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    inference_service_configuration: InferenceServiceConfiguration,
    params: TParams,
    transformer: TTransformsOutgoingMessage,
    shutdown: CancellationToken,
    drain_counter: Arc<AwaitableCounter>,
) -> impl Stream<Item = TTransformsOutgoingMessage::Output>
where
    TParams: Debug + Into<AgentJsonRpcRequest> + Send + 'static,
    AgentController: HandlesAgentStreamingResponse<TParams>,
    <<AgentController as HandlesAgentStreamingResponse<TParams>>::SenderCollection as ManagesSenders>::Value: Debug + Into<OutgoingResponse> + StreamableResult,
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage + Send + Sync + 'static,
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
                params,
                request_id,
                session_controller,
                shutdown,
            )
            .await;
        }
    });

    GuardedStream::new(
        GuardedStream::new(
            UnboundedReceiverStream::new(chunk_rx),
            connection_close.drop_guard(),
        ),
        drain_counter.increment_with_guard(),
    )
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::time::Duration;

    use futures_util::StreamExt as _;

    use super::*;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

    fn inference_service_configuration() -> InferenceServiceConfiguration {
        const TIMEOUT_LONGER_THAN_ANY_TEST_RUN: Duration = Duration::from_hours(1);

        InferenceServiceConfiguration {
            addr: "127.0.0.1:0".parse().unwrap(),
            cors_allowed_hosts: Vec::new(),
            inference_item_timeout: TIMEOUT_LONGER_THAN_ANY_TEST_RUN,
        }
    }

    #[actix_web::test]
    async fn spawned_task_runs_request_from_agent_and_closes_stream_on_shutdown() {
        let pool = Arc::new(AgentControllerPool::default());
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_secs(1),
            10,
        ));

        let shutdown = CancellationToken::new();

        shutdown.cancel();

        let mut stream = Box::pin(unbounded_stream_from_agent(
            buffered_request_manager,
            inference_service_configuration(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 1,
                raw_prompt: "fixture prompt".to_owned(),
            },
            IdentityTransformer::new(),
            shutdown,
            Arc::new(AwaitableCounter::default()),
        ));

        let shutdown_chunk = stream.next().await.unwrap();

        assert_eq!(
            discriminant(&TransformResult::Chunk(String::new())),
            discriminant(&shutdown_chunk),
        );

        let chunk_text = match shutdown_chunk {
            TransformResult::Chunk(chunk_text) | TransformResult::Error(chunk_text) => chunk_text,
            TransformResult::Discard => String::new(),
        };

        assert!(chunk_text.contains("shutting down"));
        assert!(stream.next().await.is_none());
    }
}
