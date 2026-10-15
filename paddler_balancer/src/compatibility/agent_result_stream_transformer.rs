use std::marker::PhantomData;

use async_trait::async_trait;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

use crate::agent_relay_error::AgentRelayError;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;

pub struct AgentResultStreamTransformer<TResult> {
    result: PhantomData<fn() -> TResult>,
}

impl<TResult> Default for AgentResultStreamTransformer<TResult> {
    fn default() -> Self {
        Self {
            result: PhantomData,
        }
    }
}

#[async_trait]
impl<TResult> TransformsOutgoingMessage for AgentResultStreamTransformer<TResult>
where
    TResult: TryFrom<OutgoingResponse, Error = OutgoingResponse> + Send + Sync + 'static,
{
    type Output = AgentResultStreamEvent<TResult>;

    async fn transform(
        &self,
        message: OutgoingMessage,
    ) -> Result<Vec<AgentResultStreamEvent<TResult>>, AgentRelayError> {
        match message {
            OutgoingMessage::Error(ErrorEnvelope { error, .. }) => {
                Ok(vec![AgentResultStreamEvent::WireError(error)])
            }
            notification @ OutgoingMessage::Notification(_) => {
                Err(AgentRelayError::MessageNotRelayable {
                    message: Box::new(notification),
                })
            }
            OutgoingMessage::Response(ResponseEnvelope {
                generated_by,
                request_id,
                response,
            }) => match TResult::try_from(response) {
                Ok(result) => Ok(vec![AgentResultStreamEvent::Result { request_id, result }]),
                Err(response) => Err(AgentRelayError::MessageNotRelayable {
                    message: Box::new(OutgoingMessage::Response(ResponseEnvelope {
                        generated_by,
                        request_id,
                        response,
                    })),
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::decision_result::DecisionResult;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::inference_client::notification::Notification;
    use paddler_messaging::inference_client::response::Response as OutgoingResponse;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::AgentResultStreamTransformer;
    use crate::agent_relay_error::AgentRelayError;
    use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage as _;
    use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;

    #[tokio::test]
    async fn relays_results_with_their_request_id() {
        assert_eq!(
            AgentResultStreamTransformer::<DecisionResult>::default()
                .transform(OutgoingMessage::Response(ResponseEnvelope {
                    generated_by: None,
                    request_id: "decision-request".to_owned(),
                    response: OutgoingResponse::Decision(DecisionResult::SchedulerUnavailable(
                        "unavailable".to_owned(),
                    )),
                }))
                .await
                .ok(),
            Some(vec![AgentResultStreamEvent::Result {
                request_id: "decision-request".to_owned(),
                result: DecisionResult::SchedulerUnavailable("unavailable".to_owned()),
            }])
        );
    }

    #[tokio::test]
    async fn relays_wire_errors() {
        let timed_out = || JsonRpcError {
            code: 504,
            description: "timed out".to_owned(),
        };

        assert_eq!(
            AgentResultStreamTransformer::<DecisionResult>::default()
                .transform(OutgoingMessage::Error(ErrorEnvelope {
                    error: timed_out(),
                    request_id: "decision-request".to_owned(),
                }))
                .await
                .ok(),
            Some(vec![AgentResultStreamEvent::WireError(timed_out())])
        );
    }

    #[tokio::test]
    async fn refuses_to_relay_a_response_of_another_inference_mode() {
        let embedding_response = || {
            OutgoingMessage::Response(ResponseEnvelope {
                generated_by: None,
                request_id: "embedding-request".to_owned(),
                response: OutgoingResponse::Embedding(EmbeddingResult::Done),
            })
        };

        assert_eq!(
            AgentResultStreamTransformer::<DecisionResult>::default()
                .transform(embedding_response())
                .await
                .err()
                .as_ref()
                .map(ToString::to_string),
            Some(
                AgentRelayError::MessageNotRelayable {
                    message: Box::new(embedding_response()),
                }
                .to_string()
            )
        );
    }

    #[tokio::test]
    async fn refuses_to_relay_a_cluster_notification() {
        let cluster_notification = || {
            OutgoingMessage::Notification(Notification::ClusterInferenceMode(
                InferenceMode::Decision,
            ))
        };

        assert_eq!(
            AgentResultStreamTransformer::<DecisionResult>::default()
                .transform(cluster_notification())
                .await
                .err()
                .as_ref()
                .map(ToString::to_string),
            Some(
                AgentRelayError::MessageNotRelayable {
                    message: Box::new(cluster_notification()),
                }
                .to_string()
            )
        );
    }
}
