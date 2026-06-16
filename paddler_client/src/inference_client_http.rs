use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use reqwest::Client;
use url::Url;

use crate::collect_embedding_results::collect_embedding_results;
use crate::collect_generated_tokens::collect_generated_tokens;
use crate::collected_embedding_results::CollectedEmbeddingResults;
use crate::collected_generated_tokens::CollectedGeneratedTokens;
use crate::error::Result;
use crate::format_api_url::format_api_url;
use crate::inference_message_stream::InferenceMessageStream;
use crate::stream::ndjson::Ndjson;

#[derive(Clone)]
pub struct InferenceClientHttp {
    http_client: Client,
    url: Url,
}

impl InferenceClientHttp {
    #[must_use]
    pub const fn new(url: Url, http_client: Client) -> Self {
        Self { http_client, url }
    }

    pub async fn continue_from_conversation_history(
        &self,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        self.post("/api/v1/continue_from_conversation_history", params)
            .await
    }

    pub async fn continue_from_conversation_history_collected(
        self,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> anyhow::Result<CollectedGeneratedTokens> {
        collect_generated_tokens(
            self.continue_from_conversation_history(params)
                .await
                .map_err(anyhow::Error::new)?,
        )
        .await
    }

    pub async fn continue_from_raw_prompt(
        &self,
        params: &ContinueFromRawPromptParams,
    ) -> Result<InferenceMessageStream> {
        self.post("/api/v1/continue_from_raw_prompt", params).await
    }

    pub async fn continue_from_raw_prompt_collected(
        self,
        params: &ContinueFromRawPromptParams,
    ) -> anyhow::Result<CollectedGeneratedTokens> {
        collect_generated_tokens(
            self.continue_from_raw_prompt(params)
                .await
                .map_err(anyhow::Error::new)?,
        )
        .await
    }

    pub async fn generate_embedding_batch(
        &self,
        params: &GenerateEmbeddingBatchParams,
    ) -> Result<InferenceMessageStream> {
        self.post("/api/v1/generate_embedding_batch", params).await
    }

    pub async fn generate_embedding_batch_collected(
        self,
        params: &GenerateEmbeddingBatchParams,
    ) -> anyhow::Result<CollectedEmbeddingResults> {
        collect_embedding_results(
            self.generate_embedding_batch(params)
                .await
                .map_err(anyhow::Error::new)?,
        )
        .await
    }

    async fn post<TBody>(&self, path: &str, body: &TBody) -> Result<InferenceMessageStream>
    where
        TBody: serde::Serialize + Sync + ?Sized,
    {
        let response = self
            .http_client
            .post(format_api_url(&self.url, path))
            .json(body)
            .send()
            .await?
            .error_for_status()?;

        Ok(Box::pin(Ndjson::<InferenceMessage>::from_response(
            response,
        )))
    }
}
