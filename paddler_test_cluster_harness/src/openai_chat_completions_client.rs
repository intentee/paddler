use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use reqwest::Client;
use serde_json::Value;
use url::Url;

use crate::ndjson_lines_from_response::ndjson_lines_from_response;

#[derive(Clone)]
pub struct OpenAIChatCompletionsClient {
    http_client: Client,
    completions_url: Url,
}

impl OpenAIChatCompletionsClient {
    pub fn new(http_client: Client, openai_base_url: &Url) -> Result<Self> {
        Ok(Self {
            http_client,
            completions_url: openai_base_url
                .join("v1/chat/completions")
                .context("failed to build /v1/chat/completions URL")?,
        })
    }

    pub async fn post_streaming(&self, body: &Value) -> Result<Vec<Value>> {
        let response = self
            .http_client
            .post(self.completions_url.clone())
            .json(body)
            .send()
            .await
            .context("failed to POST OpenAI streaming chat completion")?
            .error_for_status()
            .context("non-success status from OpenAI streaming endpoint")?;

        let mut lines = Box::pin(ndjson_lines_from_response(response));
        let mut chunks: Vec<Value> = Vec::new();

        while let Some(line_result) = lines.next().await {
            let line = line_result?;

            chunks.push(
                serde_json::from_str(&line)
                    .with_context(|| format!("failed to parse OpenAI streaming chunk: {line}"))?,
            );
        }

        Ok(chunks)
    }

    pub async fn post_non_streaming(&self, body: &Value) -> Result<Value> {
        self.http_client
            .post(self.completions_url.clone())
            .json(body)
            .send()
            .await
            .context("failed to POST OpenAI non-streaming chat completion")?
            .error_for_status()
            .context("non-success status from OpenAI non-streaming endpoint")?
            .json::<Value>()
            .await
            .context("failed to parse OpenAI non-streaming JSON response")
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use url::Url;

    use super::OpenAIChatCompletionsClient;

    #[tokio::test]
    async fn new_errors_for_an_unbuildable_base_url() {
        let base_url = Url::parse("data:text/plain,paddler").unwrap();

        let error = OpenAIChatCompletionsClient::new(Client::new(), &base_url)
            .err()
            .unwrap();

        assert!(
            error
                .to_string()
                .contains("failed to build /v1/chat/completions URL")
        );
    }
}
