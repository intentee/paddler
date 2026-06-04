use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use futures_util::stream::unfold;
use reqwest::Response;

use crate::Result;

fn make_stream(response: Response) -> impl Stream<Item = Result<String>> + Send {
    unfold(
        (response, String::new()),
        |(mut response, mut buffer)| async move {
            loop {
                if let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    let line = line.trim_end_matches('\r');

                    if let Some(data) = line.strip_prefix("data: ") {
                        return Some((Ok(data.to_owned()), (response, buffer)));
                    }

                    continue;
                }

                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        let text = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&text);
                    }
                    Ok(None) => return None,
                    Err(err) => return Some((Err(err.into()), (response, buffer))),
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

    async fn collect_lines(
        chunks: Vec<core::result::Result<&'static str, IoError>>,
    ) -> Vec<Result<String>> {
        Sse::from_response(response_from_chunks(chunks))
            .collect()
            .await
    }

    #[tokio::test]
    async fn yields_data_payloads() {
        let lines = collect_lines(vec![Ok("data: hello\ndata: world\n")]).await;

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].as_ref().unwrap(), "hello");
        assert_eq!(lines[1].as_ref().unwrap(), "world");
    }

    #[tokio::test]
    async fn strips_trailing_carriage_return() {
        let lines = collect_lines(vec![Ok("data: hello\r\n")]).await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_ref().unwrap(), "hello");
    }

    #[tokio::test]
    async fn skips_non_data_lines() {
        let lines = collect_lines(vec![Ok("event: ping\ndata: kept\n")]).await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_ref().unwrap(), "kept");
    }

    #[tokio::test]
    async fn reassembles_a_payload_split_across_chunks() {
        let lines = collect_lines(vec![Ok("data: hel"), Ok("lo\n")]).await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_ref().unwrap(), "hello");
    }

    #[tokio::test]
    async fn empty_response_yields_no_lines() {
        let lines = collect_lines(vec![]).await;

        assert!(lines.is_empty());
    }

    #[tokio::test]
    async fn stream_error_yields_error() {
        let lines = collect_lines(vec![
            Ok("data: partial"),
            Err(IoError::new(ErrorKind::ConnectionReset, "boom")),
        ])
        .await;

        assert!(lines.iter().any(|line| line.is_err()));
    }
}
