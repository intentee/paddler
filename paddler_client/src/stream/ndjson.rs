use std::future::ready;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use futures_util::StreamExt as _;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde_json::from_str;
use tokio_util::sync::CancellationToken;

use crate::error::Error;
use crate::error::Result;
use crate::stream::decode_stream_line::decode_stream_line;
use crate::stream::stream_line::StreamLine;
use crate::stream::stream_lines::stream_lines;

fn parse_line<TItem: DeserializeOwned>(line: &str) -> Option<Result<TItem>> {
    let trimmed_line = line.trim();

    if trimmed_line.is_empty() {
        return None;
    }

    Some(
        from_str(trimmed_line).map_err(|source| Error::NdjsonLineParseFailed {
            line: trimmed_line.to_owned(),
            source,
        }),
    )
}

pub struct Ndjson<TItem> {
    inner: Pin<Box<dyn Stream<Item = Result<TItem>> + Send>>,
}

impl<TItem: DeserializeOwned + Send + 'static> Ndjson<TItem> {
    pub fn from_response(cancellation_token: CancellationToken, response: Response) -> Self {
        let items = stream_lines(cancellation_token, response).filter_map(|line_result| {
            ready(match line_result {
                Ok(StreamLine::Terminated(line)) => parse_line(&line),
                Ok(StreamLine::Unterminated(remainder)) => match decode_stream_line(remainder) {
                    Ok(line) => parse_line(&line),
                    Err(decoding_error) => Some(Err(decoding_error)),
                },
                Err(line_error) => Some(Err(line_error)),
            })
        });

        Self {
            inner: Box::pin(items),
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
    use std::io::Result as IoResult;

    use futures_util::StreamExt as _;
    use futures_util::stream::iter;
    use http::Response as HttpResponse;
    use reqwest::Body;
    use reqwest::Response;
    use serde_json::Value;
    use serde_json::json;
    use tokio::spawn;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tokio_util::sync::CancellationToken;

    use super::Ndjson;
    use crate::error::Error;
    use crate::error::Result;

    fn response_from_chunks(chunks: Vec<IoResult<&'static [u8]>>) -> Response {
        let stream = iter(chunks.into_iter().map(|chunk| chunk.map(<[u8]>::to_vec)));

        Response::from(HttpResponse::new(Body::wrap_stream(stream)))
    }

    async fn collect_items(chunks: Vec<IoResult<&'static [u8]>>) -> Vec<Result<Value>> {
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
        assert!(matches!(
            &items[0],
            Err(Error::NonUtf8StreamLine { source }) if source.as_bytes() == b"{\"a\":\"\xf0\x9f\"}"
        ));
    }

    #[tokio::test]
    async fn a_trailing_remainder_that_is_not_valid_utf8_yields_an_error() {
        let items = collect_items(vec![Ok(b"{\"a\":\"\xf0\x9f")]).await;

        assert_eq!(items.len(), 1);
        assert!(matches!(
            &items[0],
            Err(Error::NonUtf8StreamLine { source }) if source.as_bytes() == b"{\"a\":\"\xf0\x9f"
        ));
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
        assert!(matches!(&items[0], Err(Error::Http(source)) if source.is_decode()));
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
        let (chunk_tx, chunk_rx) = mpsc::unbounded_channel::<IoResult<Vec<u8>>>();
        let response = Response::from(HttpResponse::new(Body::wrap_stream(
            UnboundedReceiverStream::new(chunk_rx),
        )));
        let cancellation_token = CancellationToken::new();
        let mut stream = Ndjson::<Value>::from_response(cancellation_token.clone(), response);

        let cancelling_token = cancellation_token.clone();

        spawn(async move {
            cancelling_token.cancel();
        });

        assert!(stream.next().await.is_none());

        drop(chunk_tx);
    }
}
