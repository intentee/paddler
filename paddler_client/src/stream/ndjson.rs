use std::marker::PhantomData;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use futures_util::stream::unfold;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde_json::from_str;
use tokio_util::sync::CancellationToken;

use crate::error::Error;
use crate::error::Result;
use crate::stream::line_buffer::LineBuffer;

fn parse_line<TItem: DeserializeOwned>(line_result: Result<String>) -> Option<Result<TItem>> {
    match line_result {
        Ok(line) => {
            let trimmed_line = line.trim();

            if trimmed_line.is_empty() {
                None
            } else {
                Some(
                    from_str(trimmed_line).map_err(|source| Error::NdjsonLineParseFailed {
                        line: trimmed_line.to_owned(),
                        source,
                    }),
                )
            }
        }
        Err(decoding_error) => Some(Err(decoding_error)),
    }
}

struct StreamState<TItem> {
    cancellation_token: CancellationToken,
    is_terminated: bool,
    item_type_marker: PhantomData<TItem>,
    line_buffer: LineBuffer,
    response: Response,
}

fn make_stream<TItem: DeserializeOwned + Send + 'static>(
    cancellation_token: CancellationToken,
    response: Response,
) -> impl Stream<Item = Result<TItem>> + Send {
    unfold(
        StreamState {
            cancellation_token,
            is_terminated: false,
            item_type_marker: PhantomData::<TItem>,
            line_buffer: LineBuffer::new(),
            response,
        },
        |mut state| async move {
            if state.is_terminated || state.cancellation_token.is_cancelled() {
                return None;
            }

            loop {
                if let Some(line_result) = state.line_buffer.take_line() {
                    if let Some(item_result) = parse_line(line_result) {
                        return Some((item_result, state));
                    }

                    continue;
                }

                let chunk_result = state
                    .cancellation_token
                    .run_until_cancelled(state.response.chunk())
                    .await?;

                match chunk_result {
                    Ok(Some(chunk)) => state.line_buffer.push_chunk(&chunk),
                    Ok(None) => {
                        let remainder_result = state.line_buffer.take_remainder()?;
                        let item_result = parse_line(remainder_result)?;

                        return Some((item_result, state));
                    }
                    Err(transport_error) => {
                        state.is_terminated = true;

                        return Some((Err(transport_error.into()), state));
                    }
                }
            }
        },
    )
}

pub struct Ndjson<TItem> {
    inner: Pin<Box<dyn Stream<Item = Result<TItem>> + Send>>,
}

impl<TItem: DeserializeOwned + Send + 'static> Ndjson<TItem> {
    pub fn from_response(cancellation_token: CancellationToken, response: Response) -> Self {
        let stream = make_stream::<TItem>(cancellation_token, response);

        Self {
            inner: Box::pin(stream),
        }
    }
}

impl<TItem> Stream for Ndjson<TItem> {
    type Item = Result<TItem>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error as IoError;
    use std::io::ErrorKind;

    use futures_util::StreamExt as _;
    use serde_json::Value;
    use serde_json::json;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tokio_util::sync::CancellationToken;

    use super::Ndjson;
    use crate::error::Error;
    use crate::error::Result;

    fn response_from_chunks(
        chunks: Vec<core::result::Result<&'static [u8], IoError>>,
    ) -> reqwest::Response {
        let stream =
            futures_util::stream::iter(chunks.into_iter().map(|chunk| chunk.map(<[u8]>::to_vec)));

        reqwest::Response::from(http::Response::new(reqwest::Body::wrap_stream(stream)))
    }

    async fn collect_items(
        chunks: Vec<core::result::Result<&'static [u8], IoError>>,
    ) -> Vec<Result<Value>> {
        Ndjson::<Value>::from_response(CancellationToken::new(), response_from_chunks(chunks))
            .collect()
            .await
    }

    #[tokio::test]
    async fn reassembles_a_multibyte_character_split_across_chunks() {
        let items = collect_items(vec![Ok(b"{\"a\":\"\xf0\x9f"), Ok(b"\xa6\x86\"}\n")]).await;

        assert_eq!(items.len(), 1);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": "🦆" }));
    }

    #[tokio::test]
    async fn reassembles_a_multibyte_character_split_across_the_trailing_remainder() {
        let items = collect_items(vec![Ok(b"{\"a\":\"\xf0\x9f"), Ok(b"\xa6\x86\"}")]).await;

        assert_eq!(items.len(), 1);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": "🦆" }));
    }

    #[tokio::test]
    async fn a_line_that_is_not_valid_utf8_yields_an_error() {
        let items = collect_items(vec![Ok(b"{\"a\":\"\xf0\x9f\"}\n")]).await;

        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], Err(Error::NonUtf8StreamLine { .. })));
    }

    #[tokio::test]
    async fn a_trailing_remainder_that_is_not_valid_utf8_yields_an_error() {
        let items = collect_items(vec![Ok(b"{\"a\":\"\xf0\x9f")]).await;

        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], Err(Error::NonUtf8StreamLine { .. })));
    }

    #[tokio::test]
    async fn parses_multiple_lines_in_one_chunk() {
        let items = collect_items(vec![Ok(b"{\"a\":1}\n{\"a\":2}\n")]).await;

        assert_eq!(items.len(), 2);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": 1 }));
        assert_eq!(*items[1].as_ref().unwrap(), json!({ "a": 2 }));
    }

    #[tokio::test]
    async fn reassembles_a_line_split_across_chunks() {
        let items = collect_items(vec![Ok(b"{\"a\""), Ok(b":1}\n")]).await;

        assert_eq!(items.len(), 1);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": 1 }));
    }

    #[tokio::test]
    async fn skips_blank_lines() {
        let items = collect_items(vec![Ok(b"\n   \n{\"a\":1}\n")]).await;

        assert_eq!(items.len(), 1);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": 1 }));
    }

    #[tokio::test]
    async fn parses_trailing_remainder_without_newline() {
        let items = collect_items(vec![Ok(b"{\"a\":1}")]).await;

        assert_eq!(items.len(), 1);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": 1 }));
    }

    #[tokio::test]
    async fn skips_a_whitespace_only_trailing_remainder() {
        let items = collect_items(vec![Ok(b"{\"a\":1}\n   ")]).await;

        assert_eq!(items.len(), 1);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": 1 }));
    }

    #[tokio::test]
    async fn empty_response_yields_no_items() {
        let items = collect_items(vec![]).await;

        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn a_malformed_line_yields_an_error_carrying_the_offending_line() {
        let items = collect_items(vec![Ok(b"not json\n")]).await;

        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0],
            Err(Error::NdjsonLineParseFailed { ref line, .. }) if line == "not json"
        ));
    }

    #[tokio::test]
    async fn a_transport_error_ends_the_stream_after_a_single_error() {
        let items = collect_items(vec![
            Ok(b"{\"a\""),
            Err(IoError::new(ErrorKind::ConnectionReset, "boom")),
        ])
        .await;

        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], Err(Error::Http(_))));
    }

    #[tokio::test]
    async fn cancelling_the_token_ends_the_stream_without_yielding_buffered_items() {
        let cancellation_token = CancellationToken::new();
        let mut stream = Ndjson::<Value>::from_response(
            cancellation_token.clone(),
            response_from_chunks(vec![Ok(b"{\"a\":1}\n{\"a\":2}\n")]),
        );

        let first_item = stream
            .next()
            .await
            .expect("the first item must be produced")
            .expect("the first item must parse");

        assert_eq!(first_item, json!({ "a": 1 }));

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
        let mut stream = Ndjson::<Value>::from_response(cancellation_token.clone(), response);

        let cancelling_token = cancellation_token.clone();

        tokio::spawn(async move {
            cancelling_token.cancel();
        });

        assert!(stream.next().await.is_none());

        drop(chunk_tx);
    }
}
