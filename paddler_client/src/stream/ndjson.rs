use std::marker::PhantomData;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use futures_util::stream::unfold;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde_json::from_str;

use crate::Result;

fn make_stream<TItem: DeserializeOwned + Send + 'static>(
    response: Response,
) -> impl Stream<Item = Result<TItem>> + Send {
    unfold(
        (response, String::new(), PhantomData::<TItem>),
        |(mut response, mut buffer, _item_type_marker)| async move {
            loop {
                if let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_owned();
                    buffer = buffer[line_end + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    let result: Result<TItem> = from_str(&line).map_err(Into::into);

                    return Some((result, (response, buffer, PhantomData)));
                }

                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        let text = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&text);
                    }
                    Ok(None) => {
                        let remaining = buffer.trim().to_owned();
                        if !remaining.is_empty() {
                            buffer.clear();
                            let result: Result<TItem> = from_str(&remaining).map_err(Into::into);

                            return Some((result, (response, buffer, PhantomData)));
                        }

                        return None;
                    }
                    Err(err) => {
                        return Some((Err(err.into()), (response, buffer, PhantomData)));
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
    pub fn from_response(response: Response) -> Self {
        let stream = make_stream::<TItem>(response);

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

    use super::Ndjson;
    use crate::Result;

    fn response_from_chunks(
        chunks: Vec<core::result::Result<&'static str, IoError>>,
    ) -> reqwest::Response {
        let stream = futures_util::stream::iter(
            chunks
                .into_iter()
                .map(|chunk| chunk.map(|text| text.as_bytes().to_vec())),
        );

        reqwest::Response::from(http::Response::new(reqwest::Body::wrap_stream(stream)))
    }

    async fn collect_items(
        chunks: Vec<core::result::Result<&'static str, IoError>>,
    ) -> Vec<Result<Value>> {
        Ndjson::<Value>::from_response(response_from_chunks(chunks))
            .collect()
            .await
    }

    #[tokio::test]
    async fn parses_multiple_lines_in_one_chunk() {
        let items = collect_items(vec![Ok("{\"a\":1}\n{\"a\":2}\n")]).await;

        assert_eq!(items.len(), 2);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": 1 }));
        assert_eq!(*items[1].as_ref().unwrap(), json!({ "a": 2 }));
    }

    #[tokio::test]
    async fn reassembles_a_line_split_across_chunks() {
        let items = collect_items(vec![Ok("{\"a\""), Ok(":1}\n")]).await;

        assert_eq!(items.len(), 1);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": 1 }));
    }

    #[tokio::test]
    async fn skips_blank_lines() {
        let items = collect_items(vec![Ok("\n   \n{\"a\":1}\n")]).await;

        assert_eq!(items.len(), 1);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": 1 }));
    }

    #[tokio::test]
    async fn parses_trailing_remainder_without_newline() {
        let items = collect_items(vec![Ok("{\"a\":1}")]).await;

        assert_eq!(items.len(), 1);
        assert_eq!(*items[0].as_ref().unwrap(), json!({ "a": 1 }));
    }

    #[tokio::test]
    async fn empty_response_yields_no_items() {
        let items = collect_items(vec![]).await;

        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn malformed_line_yields_error() {
        let items = collect_items(vec![Ok("not json\n")]).await;

        assert_eq!(items.len(), 1);
        assert!(items[0].is_err());
    }

    #[tokio::test]
    async fn stream_error_yields_error() {
        let items = collect_items(vec![
            Ok("{\"a\""),
            Err(IoError::new(ErrorKind::ConnectionReset, "boom")),
        ])
        .await;

        assert!(items.iter().any(|item| item.is_err()));
    }
}
