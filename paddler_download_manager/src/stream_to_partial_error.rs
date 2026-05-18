use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StreamToPartialError {
    #[error("stream error: {0}")]
    Stream(#[source] reqwest::Error),

    #[error("write error: {0}")]
    Write(#[source] io::Error),
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test setup primitives must not fail on a healthy CI box; an unexpected error here is an environmental problem"
)]
mod tests {
    use crate::stream_to_partial_error::StreamToPartialError;

    #[test]
    fn write_variant_formats_with_source_message() {
        let write_err = StreamToPartialError::Write(std::io::Error::other("disk full"));

        let formatted = format!("{write_err}");

        assert!(formatted.starts_with("write error:"));
    }

    #[tokio::test]
    async fn stream_variant_formats_with_source_message() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a tempo listener");
        let port = listener
            .local_addr()
            .expect("local_addr of the listener")
            .port();
        drop(listener);
        let reqwest_err = reqwest::get(format!("http://127.0.0.1:{port}/never-listens"))
            .await
            .expect_err("unreachable port must produce a reqwest error");
        let stream_err = StreamToPartialError::Stream(reqwest_err);

        let formatted = format!("{stream_err}");

        assert!(formatted.starts_with("stream error:"));
    }
}
