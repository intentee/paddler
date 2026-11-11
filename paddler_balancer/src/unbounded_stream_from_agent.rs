use std::sync::Arc;
use std::time::Duration;

use actix_web::rt;
use futures_util::Stream;
use nanoid::nanoid;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;

use crate::agent_relay::AgentRelay;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::awaiting_agent_relay::AwaitingAgentRelay;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::cancellation_token_stream_guard::CancellationTokenStreamGuard;
use crate::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::dispatchable_request::DispatchableRequest;
use crate::outgoing_message_relay_receiver::OutgoingMessageRelayReceiver;

pub struct UnboundedStreamFromAgent<TParams, TTransformsOutgoingMessage> {
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub inference_item_timeout: Duration,
    pub request_params: TParams,
    pub shutdown: CancellationToken,
    pub transformer: TTransformsOutgoingMessage,
}

impl<TParams, TTransformsOutgoingMessage>
    UnboundedStreamFromAgent<TParams, TTransformsOutgoingMessage>
where
    TParams: AgentStreamingRequest + 'static,
    TTransformsOutgoingMessage: TransformsOutgoingMessage + Send + Sync + 'static,
{
    pub fn into_stream(self) -> impl Stream<Item = TTransformsOutgoingMessage::Output> {
        let Self {
            buffered_request_manager,
            inference_item_timeout,
            request_params,
            shutdown,
            transformer,
        } = self;
        let connection_close = CancellationToken::new();
        let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

        rt::spawn(
            AgentRelay::awaiting(AwaitingAgentRelay {
                buffered_request_manager,
                dispatchable_request: DispatchableRequest::from(request_params),
                inference_item_timeout,
                shutdown,
            })
            .forward(
                connection_close.clone(),
                OutgoingMessageRelayReceiver {
                    request_id: nanoid!(),
                    session_controller: ChunkForwardingSessionController::new(
                        chunk_tx,
                        transformer,
                    ),
                },
            ),
        );

        CancellationTokenStreamGuard::new(connection_close, UnboundedReceiverStream::new(chunk_rx))
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use std::time::Duration;

    use futures_util::StreamExt as _;
    use serde_json::from_str;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

    use super::UnboundedStreamFromAgent;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;

    const TIMEOUT_LONGER_THAN_ANY_TEST_RUN: Duration = Duration::from_hours(1);

    #[actix_web::test]
    async fn a_stream_requested_during_shutdown_yields_a_shutting_down_error_and_ends() {
        let pool = Arc::new(AgentControllerPool::default());
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_secs(1),
            10,
        ));

        let shutdown = CancellationToken::new();

        shutdown.cancel();

        let mut stream = Box::pin(
            UnboundedStreamFromAgent {
                buffered_request_manager,
                inference_item_timeout: TIMEOUT_LONGER_THAN_ANY_TEST_RUN,
                request_params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::new(1).unwrap(),
                    raw_prompt: "fixture prompt".to_owned(),
                },
                shutdown,
                transformer: IdentityTransformer::new(),
            }
            .into_stream(),
        );

        let shutdown_chunk = match stream.next().await {
            Some(TransformResult::Chunk(chunk)) => Some(chunk),
            Some(TransformResult::Discard | TransformResult::Error(_)) | None => None,
        }
        .expect("the stream must forward the shutdown as a chunk");

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
