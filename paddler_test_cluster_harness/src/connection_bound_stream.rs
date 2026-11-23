use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use futures_util::StreamExt as _;

use crate::connection_keep_alive::ConnectionKeepAlive;

pub struct ConnectionBoundStream<TStream> {
    pub connection_keep_alive: ConnectionKeepAlive,
    pub stream: TStream,
}

impl<TStream: Stream + Unpin> Stream for ConnectionBoundStream<TStream> {
    type Item = TStream::Item;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.poll_next_unpin(context)
    }
}
