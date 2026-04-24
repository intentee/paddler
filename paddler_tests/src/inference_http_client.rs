use anyhow::Context as _;
use anyhow::Result;
use async_stream::try_stream;
use futures_util::Stream;
use futures_util::StreamExt as _;
use paddler_types::inference_client::Message as InferenceMessage;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::ContinueFromRawPromptParams;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use reqwest::Client;
use url::Url;

use crate::inference_message_stream::InferenceMessageStream;

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

        Ok(Box::pin(inference_messages_from_response(response)))
    }
}

fn inference_messages_from_response(
    response: reqwest::Response,
) -> impl Stream<Item = Result<InferenceMessage>> + Send {
    try_stream! {
        let mut bytes_stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();

        while let Some(chunk_result) = bytes_stream.next().await {
            let chunk = chunk_result.context("failed to read inference response chunk")?;

            buffer.extend_from_slice(&chunk);

            while let Some(newline_position) = buffer.iter().position(|byte| *byte == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..=newline_position).collect();
                let line_without_newline = &line_bytes[..newline_position];
                let line_text = std::str::from_utf8(line_without_newline)
                    .context("inference stream produced non-UTF8 bytes")?
                    .trim();

                if line_text.is_empty() {
                    continue;
                }

                let message: InferenceMessage = serde_json::from_str(line_text)
                    .with_context(|| format!("failed to parse NDJSON line: {line_text}"))?;

                yield message;
            }
        }

        let trailing_text = std::str::from_utf8(&buffer)
            .context("inference stream produced trailing non-UTF8 bytes")?
            .trim();

        if !trailing_text.is_empty() {
            let message: InferenceMessage = serde_json::from_str(trailing_text)
                .with_context(|| format!("failed to parse trailing NDJSON line: {trailing_text}"))?;

            yield message;
        }
    }
}
