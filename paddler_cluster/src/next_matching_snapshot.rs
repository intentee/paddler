use std::pin::Pin;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::Stream;
use futures_util::StreamExt as _;

pub async fn next_matching_snapshot<TSnapshot, TPredicate>(
    stream: &mut Pin<Box<dyn Stream<Item = Result<TSnapshot>> + Send>>,
    mut predicate: TPredicate,
) -> Result<TSnapshot>
where
    TPredicate: FnMut(&TSnapshot) -> bool,
{
    while let Some(item) = stream.next().await {
        let snapshot = item.context("stream yielded an error")?;

        if predicate(&snapshot) {
            return Ok(snapshot);
        }
    }

    Err(anyhow!("stream closed before predicate was satisfied"))
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;

    use anyhow::Result;
    use anyhow::anyhow;
    use futures_util::Stream;

    use super::next_matching_snapshot;

    fn stream(items: Vec<Result<i32>>) -> Pin<Box<dyn Stream<Item = Result<i32>> + Send>> {
        Box::pin(futures_util::stream::iter(items))
    }

    #[tokio::test]
    async fn returns_the_first_snapshot_satisfying_the_predicate() {
        let mut source = stream(vec![Ok(5), Ok(0)]);

        let matched = next_matching_snapshot(&mut source, |value| *value == 0)
            .await
            .unwrap();

        assert_eq!(matched, 0);
    }

    #[tokio::test]
    async fn errors_when_the_stream_closes_before_the_predicate_is_satisfied() {
        let mut source = stream(vec![Ok(5)]);

        let error = next_matching_snapshot(&mut source, |_| false)
            .await
            .err()
            .unwrap();

        assert!(error.to_string().contains("closed before predicate"));
    }

    #[tokio::test]
    async fn propagates_a_stream_error() {
        let mut source = stream(vec![Err(anyhow!("socket closed"))]);

        let error = next_matching_snapshot(&mut source, |_| true)
            .await
            .err()
            .unwrap();

        assert!(error.to_string().contains("yielded an error"));
    }
}
