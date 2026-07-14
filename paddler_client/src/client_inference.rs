use std::sync::Arc;

use nanoid::nanoid;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::client_inference_params::ClientInferenceParams;
use crate::error::Result;
use crate::http_client::HttpClient;
use crate::inference_message_stream::InferenceMessageStream;
use crate::inference_socket::pool::Pool;
use crate::reports_health::ReportsHealth;
use crate::stream::ndjson::Ndjson;

#[derive(Clone)]
pub struct ClientInference {
    http_client: HttpClient,
    inference_socket_pool: Arc<Pool>,
}

impl ClientInference {
    #[must_use]
    pub fn new(
        ClientInferenceParams {
            inference_socket_pool_size,
            url,
        }: ClientInferenceParams,
    ) -> Self {
        let inference_socket_pool = Pool::new(url.clone(), inference_socket_pool_size);

        Self {
            http_client: HttpClient::new(url),
            inference_socket_pool: Arc::new(inference_socket_pool),
        }
    }

    async fn post_streaming<TParams: Serialize + Sync + ?Sized>(
        &self,
        path: &str,
        params: &TParams,
    ) -> Result<InferenceMessageStream> {
        let response = self.http_client.post_json(path, params).await?;

        Ok(Box::pin(Ndjson::<InferenceMessage>::from_response(
            response,
        )))
    }

    async fn send_over_inference_socket(
        &self,
        request: InferenceServerRequest<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        let request_id = nanoid!();
        let message: InferenceServerMessage<ValidatedParametersSchema> =
            InferenceServerMessage::Request(RequestEnvelope {
                id: request_id.clone(),
                request,
            });
        let response_rx = self
            .inference_socket_pool
            .send_request(request_id, message)
            .await?;

        Ok(Box::pin(UnboundedReceiverStream::new(response_rx)))
    }

    #[must_use]
    pub fn subscribe_to_token_generation_mode(&self) -> broadcast::Receiver<Notification> {
        self.inference_socket_pool.subscribe_to_notifications()
    }

    pub async fn continue_from_conversation_history(
        &self,
        params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        self.send_over_inference_socket(InferenceServerRequest::ContinueFromConversationHistory(
            params,
        ))
        .await
    }

    pub async fn continue_from_raw_prompt(
        &self,
        params: ContinueFromRawPromptParams,
    ) -> Result<InferenceMessageStream> {
        self.send_over_inference_socket(InferenceServerRequest::ContinueFromRawPrompt(params))
            .await
    }

    pub async fn post_continue_from_conversation_history(
        &self,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        self.post_streaming("/api/v1/continue_from_conversation_history", params)
            .await
    }

    pub async fn post_continue_from_raw_prompt(
        &self,
        params: &ContinueFromRawPromptParams,
    ) -> Result<InferenceMessageStream> {
        self.post_streaming("/api/v1/continue_from_raw_prompt", params)
            .await
    }

    pub async fn post_generate_embedding_batch(
        &self,
        params: &GenerateEmbeddingBatchParams,
    ) -> Result<InferenceMessageStream> {
        self.post_streaming("/api/v1/generate_embedding_batch", params)
            .await
    }
}

impl ReportsHealth for ClientInference {
    fn http_client(&self) -> &HttpClient {
        &self.http_client
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
    use url::Url;

    use super::ClientInference;
    use crate::client_inference_params::ClientInferenceParams;

    fn unreachable_client() -> ClientInference {
        ClientInference::new(ClientInferenceParams {
            inference_socket_pool_size: NonZeroUsize::MIN,
            url: Url::parse("http://127.0.0.1:1").expect("the test URL must be valid"),
        })
    }

    fn raw_prompt_params() -> ContinueFromRawPromptParams {
        ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 16,
            raw_prompt: "hello".to_owned(),
        }
    }

    fn conversation_history_params()
    -> ContinueFromConversationHistoryParams<ValidatedParametersSchema> {
        ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(Vec::new()),
            enable_thinking: false,
            grammar: None,
            max_tokens: 16,
            parse_tool_calls: false,
            tools: Vec::new(),
        }
    }

    #[tokio::test]
    async fn continue_from_raw_prompt_errors_for_an_unreachable_server() {
        assert!(
            unreachable_client()
                .continue_from_raw_prompt(raw_prompt_params())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn continue_from_conversation_history_errors_for_an_unreachable_server() {
        assert!(
            unreachable_client()
                .continue_from_conversation_history(conversation_history_params())
                .await
                .is_err()
        );
    }
}
