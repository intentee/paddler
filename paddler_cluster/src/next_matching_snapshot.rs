use std::pin::Pin;

use anyhow::Result;
use futures_util::Stream;
use futures_util::StreamExt as _;

use crate::error::ClusterError;

pub async fn next_matching_snapshot<TSnapshot, TPredicate>(
    stream: &mut Pin<Box<dyn Stream<Item = Result<TSnapshot>> + Send>>,
    mut predicate: TPredicate,
) -> std::result::Result<TSnapshot, ClusterError>
where
    TPredicate: FnMut(&TSnapshot) -> bool,
{
    while let Some(item) = stream.next().await {
        let snapshot = item.map_err(|source| ClusterError::SnapshotStreamYielded { source })?;

        if predicate(&snapshot) {
            return Ok(snapshot);
        }
    }

    Err(ClusterError::SnapshotStreamClosed)
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;

    use anyhow::Result;
    use anyhow::anyhow;
    use futures_util::Stream;

    use super::next_matching_snapshot;
    use crate::error::ClusterError;

    fn stream(items: Vec<Result<i32>>) -> Pin<Box<dyn Stream<Item = Result<i32>> + Send>> {
        Box::pin(futures_util::stream::iter(items))
    }

    #[tokio::test]
    async fn returns_the_first_snapshot_satisfying_the_predicate() {
        let mut source = stream(vec![Ok(5), Ok(0)]);

        let matched = next_matching_snapshot(&mut source, |value| *value == 0).await;

        assert!(matches!(matched, Ok(0)));
    }

    #[tokio::test]
    async fn errors_when_the_stream_closes_before_the_predicate_is_satisfied() {
        let mut source = stream(vec![Ok(5)]);

        let outcome = next_matching_snapshot(&mut source, |_| false).await;

        assert!(matches!(outcome, Err(ClusterError::SnapshotStreamClosed)));
    }

    #[tokio::test]
    async fn propagates_a_stream_error() {
        let mut source = stream(vec![Err(anyhow!("socket closed"))]);

        let outcome = next_matching_snapshot(&mut source, |_| true).await;

        assert!(matches!(
            outcome,
            Err(ClusterError::SnapshotStreamYielded { .. })
        ));
    }
}
