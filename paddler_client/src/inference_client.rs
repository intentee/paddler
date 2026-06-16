use std::sync::OnceLock;

use reqwest::Client;
use url::Url;

use crate::error::Result;
use crate::format_api_url::format_api_url;
use crate::inference_client_http::InferenceClientHttp;
use crate::inference_client_params::InferenceClientParams;
use crate::inference_client_socket::InferenceClientSocket;
use crate::inference_socket::pool::Pool;

pub struct InferenceClient {
    http_client: Client,
    socket_pool: OnceLock<Pool>,
    socket_pool_size: usize,
    url: Url,
}

impl InferenceClient {
    #[must_use]
    pub fn new(
        InferenceClientParams {
            socket_pool_size,
            url,
        }: InferenceClientParams,
    ) -> Self {
        Self {
            http_client: Client::new(),
            socket_pool: OnceLock::new(),
            socket_pool_size,
            url,
        }
    }

    #[must_use]
    pub fn http(&self) -> InferenceClientHttp {
        InferenceClientHttp::new(self.url.clone(), self.http_client.clone())
    }

    #[must_use]
    pub fn socket(&self) -> InferenceClientSocket<'_> {
        InferenceClientSocket::new(
            self.socket_pool
                .get_or_init(|| Pool::new(self.url.clone(), self.socket_pool_size)),
        )
    }

    pub async fn health(&self) -> Result<String> {
        let response = self
            .http_client
            .get(format_api_url(&self.url, "/health"))
            .send()
            .await?
            .error_for_status()?;

        Ok(response.text().await?)
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
    use url::Url;

    use super::InferenceClient;
    use crate::inference_client_params::InferenceClientParams;

    fn unreachable_client() -> InferenceClient {
        InferenceClient::new(InferenceClientParams {
            socket_pool_size: 1,
            url: Url::parse("http://127.0.0.1:1").unwrap(),
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
    async fn raw_prompt_over_socket_errors_for_an_unreachable_server() {
        assert!(
            unreachable_client()
                .socket()
                .continue_from_raw_prompt(raw_prompt_params())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn conversation_history_over_socket_errors_for_an_unreachable_server() {
        assert!(
            unreachable_client()
                .socket()
                .continue_from_conversation_history(conversation_history_params())
                .await
                .is_err()
        );
    }
}
