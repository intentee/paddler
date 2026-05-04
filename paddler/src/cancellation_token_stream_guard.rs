use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use tokio_util::sync::CancellationToken;

pub struct CancellationTokenStreamGuard<TStream> {
    cancellation_token: CancellationToken,
    stream: TStream,
}

impl<TStream> CancellationTokenStreamGuard<TStream> {
    pub const fn new(stream: TStream, cancellation_token: CancellationToken) -> Self {
        Self {
            cancellation_token,
            stream,
        }
    }
}

impl<TStream> Stream for CancellationTokenStreamGuard<TStream>
where
    TStream: Stream + Unpin,
{
    type Item = TStream::Item;

    fn poll_next(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(context)
    }
}

impl<TStream> Drop for CancellationTokenStreamGuard<TStream> {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
    }
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt as _;
    use futures_util::stream;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;

    use super::*;

    #[test]
    fn dropping_wrapper_cancels_token() {
        let cancellation_token = CancellationToken::new();
        let stream = stream::empty::<()>();
        let wrapped = CancellationTokenStreamGuard::new(stream, cancellation_token.clone());

        assert!(!cancellation_token.is_cancelled());
        drop(wrapped);
        assert!(cancellation_token.is_cancelled());
    }

    #[tokio::test]
    async fn forwards_inner_stream_items() {
        let cancellation_token = CancellationToken::new();
        let (sender, receiver) = mpsc::unbounded_channel::<i32>();
        let stream = UnboundedReceiverStream::new(receiver);
        let mut wrapped = CancellationTokenStreamGuard::new(stream, cancellation_token.clone());

        assert!(sender.send(7).is_ok());
        assert!(sender.send(11).is_ok());
        drop(sender);

        assert_eq!(wrapped.next().await, Some(7));
        assert_eq!(wrapped.next().await, Some(11));
        assert_eq!(wrapped.next().await, None);
        assert!(!cancellation_token.is_cancelled());

        drop(wrapped);
        assert!(cancellation_token.is_cancelled());
    }
}
