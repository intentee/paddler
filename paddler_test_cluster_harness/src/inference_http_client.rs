use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use reqwest::Client;
use url::Url;

use crate::inference_message_stream::InferenceMessageStream;
use crate::ndjson_lines_from_response::ndjson_lines_from_response;

#[derive(Clone)]
pub struct InferenceHttpClient {
    http_client: Client,
    inference_base_url: Url,
}

impl InferenceHttpClient {
    #[must_use]
    pub const fn new(http_client: Client, inference_base_url: Url) -> Self {
        Self {
            http_client,
            inference_base_url,
        }
    }

    pub async fn post_continue_from_raw_prompt(
        &self,
        params: &ContinueFromRawPromptParams,
    ) -> Result<InferenceMessageStream> {
        self.post_streaming("api/v1/continue_from_raw_prompt", params)
            .await
    }

    pub async fn post_continue_from_conversation_history(
        &self,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        self.post_streaming("api/v1/continue_from_conversation_history", params)
            .await
    }

    pub async fn post_generate_embedding_batch(
        &self,
        params: &GenerateEmbeddingBatchParams,
    ) -> Result<InferenceMessageStream> {
        self.post_streaming("api/v1/generate_embedding_batch", params)
            .await
    }

    async fn post_streaming<TBody>(
        &self,
        relative_path: &str,
        body: &TBody,
    ) -> Result<InferenceMessageStream>
    where
        TBody: serde::Serialize + Sync + ?Sized,
    {
        let request_url = self
            .inference_base_url
            .join(relative_path)
            .with_context(|| format!("failed to build URL for {relative_path}"))?;

        let response = self
            .http_client
            .post(request_url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("failed to POST {relative_path}"))?
            .error_for_status()
            .with_context(|| format!("non-success status on {relative_path}"))?;

        Ok(Box::pin(ndjson_lines_from_response(response).map(
            |line_result| {
                let line = line_result?;

                serde_json::from_str::<InferenceMessage>(&line)
                    .with_context(|| format!("failed to parse NDJSON line: {line}"))
            },
        )))
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
    use reqwest::Client;
    use url::Url;

    use super::InferenceHttpClient;

    fn empty_embedding_batch() -> GenerateEmbeddingBatchParams {
        GenerateEmbeddingBatchParams {
            input_batch: Vec::new(),
            normalization_method: EmbeddingNormalizationMethod::None,
        }
    }

    #[tokio::test]
    async fn errors_when_the_request_url_cannot_be_built() {
        let base_url = Url::parse("data:text/plain,paddler").unwrap();
        let client = InferenceHttpClient::new(Client::new(), base_url);

        let error = client
            .post_generate_embedding_batch(&empty_embedding_batch())
            .await
            .err()
            .unwrap();

        assert!(error.to_string().contains("failed to build URL"));
    }
}
