use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::Stream;
use futures_util::StreamExt as _;
use tokio::time::timeout;

pub async fn wait_for_stream_predicate<TItem, TStream, TPredicate, TOutput>(
    mut stream: TStream,
    predicate: TPredicate,
    idle_timeout: Duration,
    description: &'static str,
) -> Result<TOutput>
where
    TStream: Stream<Item = Result<TItem>> + Unpin + Send,
    TItem: Send,
    TPredicate: Fn(&TItem) -> Option<TOutput> + Send + Sync,
    TOutput: Send,
{
    loop {
        let next_item = timeout(idle_timeout, stream.next()).await.map_err(|_| {
            anyhow!("no event for {description} in {idle_timeout:?} of stream idle time")
        })?;

        let item = next_item
            .ok_or_else(|| anyhow!("stream ended before {description}"))?
            .with_context(|| {
                format!("stream yielded an error while waiting for {description}")
            })?;

        if let Some(output) = predicate(&item) {
            return Ok(output);
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use futures_util::stream;

    use super::*;

    #[tokio::test]
    async fn returns_output_on_first_matching_item() -> Result<()> {
        let items: Vec<Result<i32>> = vec![Ok(7)];
        let result = wait_for_stream_predicate(
            stream::iter(items),
            |value: &i32| if *value == 7 { Some(*value * 2) } else { None },
            Duration::from_millis(50),
            "value of seven",
        )
        .await?;

        assert_eq!(result, 14);

        Ok(())
    }

    #[tokio::test]
    async fn skips_items_until_predicate_matches() -> Result<()> {
        let items: Vec<Result<i32>> = vec![Ok(1), Ok(2), Ok(3)];
        let result = wait_for_stream_predicate(
            stream::iter(items),
            |value: &i32| if *value == 3 { Some(*value) } else { None },
            Duration::from_millis(50),
            "value of three",
        )
        .await?;

        assert_eq!(result, 3);

        Ok(())
    }

    fn message_when_err<TValue: std::fmt::Debug>(outcome: Result<TValue>) -> Result<String> {
        match outcome {
            Ok(value) => Err(anyhow!("expected an error outcome, got Ok({value:?})")),
            Err(error) => Ok(error.to_string()),
        }
    }

    #[tokio::test]
    async fn propagates_error_from_stream_item() -> Result<()> {
        let items: Vec<Result<i32>> = vec![Err(anyhow!("upstream failure"))];
        let outcome = wait_for_stream_predicate(
            stream::iter(items),
            |_value: &i32| Some(()),
            Duration::from_millis(50),
            "never reached",
        )
        .await;

        assert!(message_when_err(outcome)?.contains("never reached"));

        Ok(())
    }

    #[tokio::test]
    async fn returns_end_of_stream_error_when_nothing_matches() -> Result<()> {
        let items: Vec<Result<i32>> = vec![Ok(1), Ok(2)];
        let outcome = wait_for_stream_predicate(
            stream::iter(items),
            |_value: &i32| Option::<()>::None,
            Duration::from_millis(50),
            "impossible match",
        )
        .await;

        assert!(message_when_err(outcome)?.contains("stream ended"));

        Ok(())
    }

    #[tokio::test]
    async fn returns_timeout_error_when_stream_is_idle() -> Result<()> {
        let empty_stream = stream::pending::<Result<i32>>();
        let outcome = wait_for_stream_predicate(
            empty_stream,
            |_value: &i32| Some(()),
            Duration::from_millis(50),
            "an event that never arrives",
        )
        .await;

        assert!(message_when_err(outcome)?.contains("idle"));

        Ok(())
    }
}
