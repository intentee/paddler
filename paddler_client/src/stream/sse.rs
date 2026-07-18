use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use futures_util::stream::unfold;
use reqwest::Response;
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::stream::line_buffer::LineBuffer;

const DATA_FIELD_PREFIX: &str = "data: ";

struct StreamState {
    cancellation_token: CancellationToken,
    is_terminated: bool,
    line_buffer: LineBuffer,
    response: Response,
}

fn make_stream(
    cancellation_token: CancellationToken,
    response: Response,
) -> impl Stream<Item = Result<String>> + Send {
    unfold(
        StreamState {
            cancellation_token,
            is_terminated: false,
            line_buffer: LineBuffer::new(),
            response,
        },
        |mut state| async move {
            if state.is_terminated || state.cancellation_token.is_cancelled() {
                return None;
            }

            loop {
                if let Some(line_result) = state.line_buffer.take_line() {
                    match line_result {
                        Ok(line) => {
                            if let Some(data) =
                                line.trim_end_matches('\r').strip_prefix(DATA_FIELD_PREFIX)
                            {
                                let data = data.to_owned();

                                return Some((Ok(data), state));
                            }
                        }
                        Err(decoding_error) => {
                            return Some((Err(decoding_error), state));
                        }
                    }

                    continue;
                }

                let chunk_result = state
                    .cancellation_token
                    .run_until_cancelled(state.response.chunk())
                    .await?;

                match chunk_result {
                    Ok(Some(chunk)) => state.line_buffer.push_chunk(&chunk),
                    Ok(None) => return None,
                    Err(transport_error) => {
                        state.is_terminated = true;

                        return Some((Err(transport_error.into()), state));
                    }
                }
            }
        },
    )
}

pub struct Sse {
    lines: Pin<Box<dyn Stream<Item = Result<String>> + Send>>,
}

impl Sse {
    pub fn from_response(cancellation_token: CancellationToken, response: Response) -> Self {
        let stream = make_stream(cancellation_token, response);

        Self {
            lines: Box::pin(stream),
        }
    }
}

impl Stream for Sse {
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.lines.as_mut().poll_next(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error as IoError;
    use std::io::ErrorKind;

    use futures_util::StreamExt as _;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tokio_util::sync::CancellationToken;

    use super::Sse;
    use crate::error::Error;
    use crate::error::Result;

    fn response_from_chunks(
        chunks: Vec<core::result::Result<&'static [u8], IoError>>,
    ) -> reqwest::Response {
        let stream =
            futures_util::stream::iter(chunks.into_iter().map(|chunk| chunk.map(<[u8]>::to_vec)));

        reqwest::Response::from(http::Response::new(reqwest::Body::wrap_stream(stream)))
    }

    async fn collect_lines(
        chunks: Vec<core::result::Result<&'static [u8], IoError>>,
    ) -> Vec<Result<String>> {
        Sse::from_response(CancellationToken::new(), response_from_chunks(chunks))
            .collect()
            .await
    }

    #[tokio::test]
    async fn yields_data_payloads() {
        let lines = collect_lines(vec![Ok(b"data: hello\ndata: world\n")]).await;

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].as_ref().unwrap(), "hello");
        assert_eq!(lines[1].as_ref().unwrap(), "world");
    }

    #[tokio::test]
    async fn reassembles_a_multibyte_character_split_across_chunks() {
        let lines = collect_lines(vec![Ok(b"data: \xf0\x9f"), Ok(b"\xa6\x86\n")]).await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_ref().unwrap(), "🦆");
    }

    #[tokio::test]
    async fn a_line_that_is_not_valid_utf8_yields_an_error() {
        let lines = collect_lines(vec![Ok(b"data: \xf0\x9f\n")]).await;

        assert_eq!(lines.len(), 1);
        assert!(matches!(lines[0], Err(Error::NonUtf8StreamLine { .. })));
    }

    #[tokio::test]
    async fn strips_trailing_carriage_return() {
        let lines = collect_lines(vec![Ok(b"data: hello\r\n")]).await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_ref().unwrap(), "hello");
    }

    #[tokio::test]
    async fn skips_non_data_lines() {
        let lines = collect_lines(vec![Ok(b"event: ping\ndata: kept\n")]).await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_ref().unwrap(), "kept");
    }

    #[tokio::test]
    async fn reassembles_a_payload_split_across_chunks() {
        let lines = collect_lines(vec![Ok(b"data: hel"), Ok(b"lo\n")]).await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_ref().unwrap(), "hello");
    }

    #[tokio::test]
    async fn discards_an_event_that_is_not_terminated_by_a_newline() {
        let lines = collect_lines(vec![Ok(b"data: kept\ndata: truncated")]).await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_ref().unwrap(), "kept");
    }

    #[tokio::test]
    async fn empty_response_yields_no_lines() {
        let lines = collect_lines(vec![]).await;

        assert!(lines.is_empty());
    }

    #[tokio::test]
    async fn a_transport_error_ends_the_stream_after_a_single_error() {
        let lines = collect_lines(vec![
            Ok(b"data: partial"),
            Err(IoError::new(ErrorKind::ConnectionReset, "boom")),
        ])
        .await;

        assert_eq!(lines.len(), 1);
        assert!(matches!(lines[0], Err(Error::Http(_))));
    }

    #[tokio::test]
    async fn cancelling_the_token_ends_the_stream_without_yielding_buffered_events() {
        let cancellation_token = CancellationToken::new();
        let mut stream = Sse::from_response(
            cancellation_token.clone(),
            response_from_chunks(vec![Ok(b"data: kept\ndata: dropped\n")]),
        );

        let first_line = stream
            .next()
            .await
            .expect("the first event must be produced")
            .expect("the first event must decode");

        assert_eq!(first_line, "kept");

        cancellation_token.cancel();

        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn cancelling_the_token_while_awaiting_a_chunk_ends_the_stream() {
        let (chunk_tx, chunk_rx) =
            mpsc::unbounded_channel::<core::result::Result<Vec<u8>, IoError>>();
        let response = reqwest::Response::from(http::Response::new(reqwest::Body::wrap_stream(
            UnboundedReceiverStream::new(chunk_rx),
        )));
        let cancellation_token = CancellationToken::new();
        let mut stream = Sse::from_response(cancellation_token.clone(), response);

        let cancelling_token = cancellation_token.clone();

        tokio::spawn(async move {
            cancelling_token.cancel();
        });

        assert!(stream.next().await.is_none());

        drop(chunk_tx);
    }
}
