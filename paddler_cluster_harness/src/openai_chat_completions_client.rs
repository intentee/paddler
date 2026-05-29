use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use reqwest::Client;
use serde_json::Value;
use url::Url;

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

        let mut bytes_stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();
        let mut chunks: Vec<Value> = Vec::new();

        while let Some(chunk_result) = bytes_stream.next().await {
            let chunk = chunk_result.context("failed to read OpenAI streaming chunk")?;

            buffer.extend_from_slice(&chunk);

            while let Some(newline_position) = buffer.iter().position(|byte| *byte == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..=newline_position).collect();
                let line_text = std::str::from_utf8(&line_bytes[..newline_position])
                    .context("OpenAI stream produced non-UTF8 bytes")?
                    .trim();

                if line_text.is_empty() {
                    continue;
                }

                chunks.push(serde_json::from_str(line_text).with_context(|| {
                    format!("failed to parse OpenAI streaming chunk: {line_text}")
                })?);
            }
        }

        let trailing_text = std::str::from_utf8(&buffer)
            .context("OpenAI stream produced trailing non-UTF8 bytes")?
            .trim();

        if !trailing_text.is_empty() {
            chunks.push(
                serde_json::from_str(trailing_text)
                    .with_context(|| format!("failed to parse trailing chunk: {trailing_text}"))?,
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
