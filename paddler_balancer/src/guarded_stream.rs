use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;

pub struct GuardedStream<TStream, TGuard> {
    _guard: TGuard,
    stream: TStream,
}

impl<TStream, TGuard> GuardedStream<TStream, TGuard> {
    pub const fn new(stream: TStream, guard: TGuard) -> Self {
        Self {
            _guard: guard,
            stream,
        }
    }
}

impl<TStream, TGuard> Stream for GuardedStream<TStream, TGuard>
where
    TStream: Stream + Unpin,
    TGuard: Unpin,
{
    type Item = TStream::Item;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(context)
    }
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt as _;
    use futures_util::stream;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tokio_util::sync::CancellationToken;

    use super::*;

    #[tokio::test]
    async fn forwards_inner_stream_items() {
        let cancellation_token = CancellationToken::new();
        let (sender, receiver) = mpsc::unbounded_channel::<i32>();
        let stream = UnboundedReceiverStream::new(receiver);
        let mut wrapped = GuardedStream::new(stream, cancellation_token.drop_guard());

        assert!(sender.send(7).is_ok());
        assert!(sender.send(11).is_ok());
        drop(sender);

        assert_eq!(wrapped.next().await, Some(7));
        assert_eq!(wrapped.next().await, Some(11));
        assert_eq!(wrapped.next().await, None);
    }

    #[test]
    fn dropping_stream_drops_owned_guard() {
        let cancellation_token = CancellationToken::new();
        let stream = stream::empty::<()>();
        let wrapped = GuardedStream::new(stream, cancellation_token.clone().drop_guard());

        assert!(!cancellation_token.is_cancelled());

        drop(wrapped);

        assert!(cancellation_token.is_cancelled());
    }
}
