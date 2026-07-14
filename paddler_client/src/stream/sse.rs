use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use futures_util::stream::unfold;
use reqwest::Response;

use crate::error::Result;
use crate::stream::line_buffer::LineBuffer;

const DATA_FIELD_PREFIX: &str = "data: ";

struct StreamState {
    is_terminated: bool,
    line_buffer: LineBuffer,
    response: Response,
}

fn make_stream(response: Response) -> impl Stream<Item = Result<String>> + Send {
    unfold(
        StreamState {
            is_terminated: false,
            line_buffer: LineBuffer::new(),
            response,
        },
        |mut state| async move {
            if state.is_terminated {
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

                match state.response.chunk().await {
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
    pub fn from_response(response: Response) -> Self {
        let stream = make_stream(response);

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
        Sse::from_response(response_from_chunks(chunks))
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
}
