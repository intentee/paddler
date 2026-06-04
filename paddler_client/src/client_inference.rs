use std::sync::OnceLock;

use nanoid::nanoid;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use reqwest::Client;
use tokio_stream::wrappers::UnboundedReceiverStream;
use url::Url;

use crate::error::Result;
use crate::format_api_url::format_api_url;
use crate::inference_message_stream::InferenceMessageStream;
use crate::inference_socket::pool::Pool;
use crate::stream::ndjson::Ndjson;

pub struct ClientInference<'client> {
    url: &'client Url,
    http_client: &'client Client,
    inference_socket_pool: OnceLock<Pool>,
    inference_socket_pool_size: usize,
}

impl<'client> ClientInference<'client> {
    #[must_use]
    pub const fn new(
        url: &'client Url,
        http_client: &'client Client,
        inference_socket_pool_size: usize,
    ) -> Self {
        Self {
            url,
            http_client,
            inference_socket_pool: OnceLock::new(),
            inference_socket_pool_size,
        }
    }

    pub async fn get_health(&self) -> Result<String> {
        let response = self
            .http_client
            .get(format_api_url(self.url, "/health"))
            .send()
            .await?
            .error_for_status()?;

        Ok(response.text().await?)
    }

    fn get_inference_socket_pool(&self) -> &Pool {
        self.inference_socket_pool
            .get_or_init(|| Pool::new(self.url.clone(), self.inference_socket_pool_size))
    }

    pub async fn continue_from_conversation_history(
        &self,
        params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        let request_id = nanoid!();
        let message: InferenceServerMessage<ValidatedParametersSchema> =
            InferenceServerMessage::Request(RequestEnvelope {
                id: request_id.clone(),
                request: InferenceServerRequest::ContinueFromConversationHistory(params),
            });
        let rx = self
            .get_inference_socket_pool()
            .send_request(request_id, message)
            .await?;

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    pub async fn continue_from_raw_prompt(
        &self,
        params: ContinueFromRawPromptParams,
    ) -> Result<InferenceMessageStream> {
        let request_id = nanoid!();
        let message: InferenceServerMessage<ValidatedParametersSchema> =
            InferenceServerMessage::Request(RequestEnvelope {
                id: request_id.clone(),
                request: InferenceServerRequest::ContinueFromRawPrompt(params),
            });
        let rx = self
            .get_inference_socket_pool()
            .send_request(request_id, message)
            .await?;

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    pub async fn post_continue_from_conversation_history(
        &self,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        let response = self
            .http_client
            .post(format_api_url(
                self.url,
                "/api/v1/continue_from_conversation_history",
            ))
            .json(params)
            .send()
            .await?
            .error_for_status()?;

        let stream = Ndjson::<InferenceMessage>::from_response(response);

        Ok(Box::pin(stream))
    }

    pub async fn post_continue_from_raw_prompt(
        &self,
        params: &ContinueFromRawPromptParams,
    ) -> Result<InferenceMessageStream> {
        let response = self
            .http_client
            .post(format_api_url(self.url, "/api/v1/continue_from_raw_prompt"))
            .json(params)
            .send()
            .await?
            .error_for_status()?;

        let stream = Ndjson::<InferenceMessage>::from_response(response);

        Ok(Box::pin(stream))
    }

    pub async fn generate_embedding_batch(
        &self,
        params: &GenerateEmbeddingBatchParams,
    ) -> Result<InferenceMessageStream> {
        let response = self
            .http_client
            .post(format_api_url(self.url, "/api/v1/generate_embedding_batch"))
            .json(params)
            .send()
            .await?
            .error_for_status()?;

        let stream = Ndjson::<InferenceMessage>::from_response(response);

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
    use reqwest::Client;
    use url::Url;

    use super::ClientInference;

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
        let url = Url::parse("http://127.0.0.1:1").unwrap();
        let http_client = Client::new();
        let inference = ClientInference::new(&url, &http_client, 1);

        assert!(
            inference
                .continue_from_raw_prompt(raw_prompt_params())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn continue_from_conversation_history_errors_for_an_unreachable_server() {
        let url = Url::parse("http://127.0.0.1:1").unwrap();
        let http_client = Client::new();
        let inference = ClientInference::new(&url, &http_client, 1);

        assert!(
            inference
                .continue_from_conversation_history(conversation_history_params())
                .await
                .is_err()
        );
    }
}
