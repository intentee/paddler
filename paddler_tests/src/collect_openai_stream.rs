use anyhow::Context as _;
use anyhow::Result;
use async_openai::error::OpenAIError;
use futures_util::Stream;
use futures_util::StreamExt as _;
use serde_json::Value;

pub async fn collect_openai_stream<TStream>(mut stream: TStream) -> Result<Vec<Value>>
where
    TStream: Stream<Item = Result<Value, OpenAIError>> + Unpin,
{
    let mut events: Vec<Value> = Vec::new();

    while let Some(event) = stream.next().await {
        events.push(event.context("OpenAI streaming event failed")?);
    }

    Ok(events)
}
