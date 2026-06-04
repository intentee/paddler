use anyhow::Context as _;
use anyhow::Result;
use async_stream::try_stream;
use futures_util::Stream;
use futures_util::StreamExt as _;

pub fn ndjson_lines_from_response(
    response: reqwest::Response,
) -> impl Stream<Item = Result<String>> + Send {
    try_stream! {
        let mut bytes_stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();

        while let Some(chunk_result) = bytes_stream.next().await {
            let chunk = chunk_result.context("failed to read response chunk")?;

            buffer.extend_from_slice(&chunk);

            while let Some(newline_position) = buffer.iter().position(|byte| *byte == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..=newline_position).collect();
                let line_text = std::str::from_utf8(&line_bytes[..newline_position])
                    .context("response stream produced non-UTF8 bytes")?
                    .trim();

                if line_text.is_empty() {
                    continue;
                }

                yield line_text.to_owned();
            }
        }

        let trailing_text = std::str::from_utf8(&buffer)
            .context("response stream produced trailing non-UTF8 bytes")?
            .trim();

        if !trailing_text.is_empty() {
            yield trailing_text.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error as IoError;

    use futures_util::StreamExt as _;

    use super::ndjson_lines_from_response;

    fn response_from_chunks(chunks: Vec<Result<Vec<u8>, IoError>>) -> reqwest::Response {
        let byte_stream = futures_util::stream::iter(chunks);
        let body = reqwest::Body::wrap_stream(byte_stream);

        reqwest::Response::from(http::Response::new(body))
    }

    async fn collect_lines(response: reqwest::Response) -> Vec<anyhow::Result<String>> {
        Box::pin(ndjson_lines_from_response(response))
            .collect()
            .await
    }

    #[tokio::test]
    async fn splits_multiple_lines_on_newlines() {
        let lines = collect_lines(response_from_chunks(vec![Ok(b"alpha\nbeta\n".to_vec())])).await;

        let collected: Vec<String> = lines.into_iter().map(anyhow::Result::unwrap).collect();

        assert_eq!(collected, vec!["alpha".to_owned(), "beta".to_owned()]);
    }

    #[tokio::test]
    async fn yields_a_trailing_line_without_a_terminating_newline() {
        let lines = collect_lines(response_from_chunks(vec![Ok(b"alpha\nbeta".to_vec())])).await;

        let collected: Vec<String> = lines.into_iter().map(anyhow::Result::unwrap).collect();

        assert_eq!(collected, vec!["alpha".to_owned(), "beta".to_owned()]);
    }

    #[tokio::test]
    async fn skips_empty_and_whitespace_only_lines() {
        let lines = collect_lines(response_from_chunks(vec![Ok(b"\n   \nalpha\n".to_vec())])).await;

        let collected: Vec<String> = lines.into_iter().map(anyhow::Result::unwrap).collect();

        assert_eq!(collected, vec!["alpha".to_owned()]);
    }

    #[tokio::test]
    async fn buffers_a_line_split_across_chunks() {
        let lines = collect_lines(response_from_chunks(vec![
            Ok(b"al".to_vec()),
            Ok(b"pha\n".to_vec()),
        ]))
        .await;

        let collected: Vec<String> = lines.into_iter().map(anyhow::Result::unwrap).collect();

        assert_eq!(collected, vec!["alpha".to_owned()]);
    }

    #[tokio::test]
    async fn errors_on_a_non_utf8_line() {
        let lines = collect_lines(response_from_chunks(vec![Ok(vec![0xff, 0xfe, b'\n'])])).await;

        let error = lines.into_iter().next().unwrap().err().unwrap();

        assert!(error.to_string().contains("non-UTF8"));
    }

    #[tokio::test]
    async fn errors_on_non_utf8_trailing_bytes() {
        let lines = collect_lines(response_from_chunks(vec![Ok(vec![0xff, 0xfe])])).await;

        let error = lines.into_iter().next().unwrap().err().unwrap();

        assert!(error.to_string().contains("trailing non-UTF8"));
    }

    #[tokio::test]
    async fn errors_when_a_chunk_fails_to_read() {
        let lines = collect_lines(response_from_chunks(vec![Err(IoError::other("boom"))])).await;

        let error = lines.into_iter().next().unwrap().err().unwrap();

        assert!(error.to_string().contains("failed to read response chunk"));
    }
}
