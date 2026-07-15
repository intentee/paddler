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
use tokio_util::sync::CancellationToken;

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
        cancellation_token: CancellationToken,
        path: &str,
        params: &TParams,
    ) -> Result<InferenceMessageStream> {
        let response = self
            .http_client
            .post_json(cancellation_token.clone(), path, params)
            .await?;

        Ok(Box::pin(Ndjson::<InferenceMessage>::from_response(
            cancellation_token,
            response,
        )))
    }

    async fn send_over_inference_socket(
        &self,
        cancellation_token: CancellationToken,
        request: InferenceServerRequest<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        let request_id = nanoid!();
        let message: InferenceServerMessage<ValidatedParametersSchema> =
            InferenceServerMessage::Request(RequestEnvelope {
                id: request_id.clone(),
                request,
            });

        self.inference_socket_pool
            .send_request(cancellation_token, request_id, message)
            .await
    }

    #[must_use]
    pub fn subscribe_to_token_generation_mode(&self) -> broadcast::Receiver<Notification> {
        self.inference_socket_pool.subscribe_to_notifications()
    }

    pub async fn continue_from_conversation_history(
        &self,
        cancellation_token: CancellationToken,
        params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        self.send_over_inference_socket(
            cancellation_token,
            InferenceServerRequest::ContinueFromConversationHistory(params),
        )
        .await
    }

    pub async fn continue_from_raw_prompt(
        &self,
        cancellation_token: CancellationToken,
        params: ContinueFromRawPromptParams,
    ) -> Result<InferenceMessageStream> {
        self.send_over_inference_socket(
            cancellation_token,
            InferenceServerRequest::ContinueFromRawPrompt(params),
        )
        .await
    }

    pub async fn post_continue_from_conversation_history(
        &self,
        cancellation_token: CancellationToken,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        self.post_streaming(
            cancellation_token,
            "/api/v1/continue_from_conversation_history",
            params,
        )
        .await
    }

    pub async fn post_continue_from_raw_prompt(
        &self,
        cancellation_token: CancellationToken,
        params: &ContinueFromRawPromptParams,
    ) -> Result<InferenceMessageStream> {
        self.post_streaming(
            cancellation_token,
            "/api/v1/continue_from_raw_prompt",
            params,
        )
        .await
    }

    pub async fn post_generate_embedding_batch(
        &self,
        cancellation_token: CancellationToken,
        params: &GenerateEmbeddingBatchParams,
    ) -> Result<InferenceMessageStream> {
        self.post_streaming(
            cancellation_token,
            "/api/v1/generate_embedding_batch",
            params,
        )
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
    use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
    use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::ClientInference;
    use crate::client_inference_params::ClientInferenceParams;
    use crate::error::Error;

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

    fn embedding_batch_params() -> GenerateEmbeddingBatchParams {
        GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "hello".to_owned(),
                id: "document-0".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        }
    }

    #[tokio::test]
    async fn continue_from_raw_prompt_errors_for_an_unreachable_server() {
        assert!(
            unreachable_client()
                .continue_from_raw_prompt(CancellationToken::new(), raw_prompt_params())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn continue_from_conversation_history_errors_for_an_unreachable_server() {
        assert!(
            unreachable_client()
                .continue_from_conversation_history(
                    CancellationToken::new(),
                    conversation_history_params(),
                )
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn an_already_cancelled_token_rejects_an_inference_request() {
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        assert!(matches!(
            unreachable_client()
                .continue_from_raw_prompt(cancellation_token, raw_prompt_params())
                .await,
            Err(Error::InferenceRequestCancelled { .. })
        ));
    }

    #[tokio::test]
    async fn post_continue_from_raw_prompt_errors_for_an_unreachable_server() {
        assert!(
            unreachable_client()
                .post_continue_from_raw_prompt(CancellationToken::new(), &raw_prompt_params())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn post_continue_from_conversation_history_errors_for_an_unreachable_server() {
        assert!(
            unreachable_client()
                .post_continue_from_conversation_history(
                    CancellationToken::new(),
                    &conversation_history_params(),
                )
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn post_generate_embedding_batch_errors_for_an_unreachable_server() {
        assert!(
            unreachable_client()
                .post_generate_embedding_batch(CancellationToken::new(), &embedding_batch_params())
                .await
                .is_err()
        );
    }
}
