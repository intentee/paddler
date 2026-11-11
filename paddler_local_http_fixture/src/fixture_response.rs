use std::future::pending;
use std::io;
use std::time::Duration;

use http::StatusCode;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpStream;
use tokio::time::interval;

use crate::observed_requests::ObservedRequests;
use crate::read_range_header::read_range_header;

fn hugging_face_file_headers(commit_hash: &str, etag: &str) -> String {
    format!("ETag: \"{etag}\"\r\nX-Repo-Commit: {commit_hash}\r\n")
}

fn status_line_and_headers(
    status: StatusCode,
    content_length: usize,
    extra_headers: &str,
) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {content_length}\r\n{extra_headers}Connection: close\r\n\r\n"
    )
}

const ENDLESS_CHUNKED_RESPONSE_HEAD: &str =
    "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
const ONE_BYTE_CHUNK: &[u8] = b"1\r\n\0\r\n";

async fn trickle_until_the_client_stops_receiving(
    stream: &mut TcpStream,
    byte_interval: Duration,
) -> io::Error {
    let mut trickle = interval(byte_interval);
    let mut next_trickled_bytes = ENDLESS_CHUNKED_RESPONSE_HEAD.as_bytes();

    loop {
        trickle.tick().await;

        if let Err(client_departure) = stream.write_all(next_trickled_bytes).await {
            return client_departure;
        }

        next_trickled_bytes = ONE_BYTE_CHUNK;
    }
}

async fn write_complete_body(stream: &mut TcpStream, head: &str, body: &[u8]) -> io::Result<()> {
    stream.write_all(&[head.as_bytes(), body].concat()).await?;
    stream.shutdown().await
}

async fn write_partial_content(
    stream: &mut TcpStream,
    declared_content_length: usize,
    content_range: &str,
    body: &[u8],
) -> io::Result<()> {
    write_complete_body(
        stream,
        &status_line_and_headers(
            StatusCode::PARTIAL_CONTENT,
            declared_content_length,
            &format!("Content-Range: {content_range}\r\n"),
        ),
        body,
    )
    .await
}

async fn write_body_prefix(
    stream: &mut TcpStream,
    status: StatusCode,
    extra_headers: &str,
    declared_body: &[u8],
    sent_byte_count: usize,
) -> io::Result<()> {
    let head_and_sent_bytes: Vec<u8> =
        status_line_and_headers(status, declared_body.len(), extra_headers)
            .bytes()
            .chain(declared_body.iter().copied().take(sent_byte_count))
            .collect();

    stream.write_all(&head_and_sent_bytes).await
}

#[derive(Clone)]
pub enum FixtureResponse {
    EndlessTrickle {
        byte_interval: Duration,
    },
    HuggingFaceFile {
        body: Vec<u8>,
        commit_hash: String,
        etag: String,
    },
    Ok(Vec<u8>),
    PartialContent {
        body: Vec<u8>,
        content_range: String,
    },
    PartialContentDeclaringLength {
        content_range: String,
        declared_content_length: usize,
    },
    StallBeforeHeaders,
    StalledBody {
        declared_body: Vec<u8>,
        sent_byte_count: usize,
    },
    Status(StatusCode),
    TruncatedBody {
        declared_body: Vec<u8>,
        sent_byte_count: usize,
        status: StatusCode,
    },
    TruncatedHuggingFaceFile {
        commit_hash: String,
        declared_body: Vec<u8>,
        etag: String,
        sent_byte_count: usize,
    },
}

impl FixtureResponse {
    /// # Errors
    ///
    /// Returns the [`io::Error`] of reading the request when it is malformed or the client leaves before it ends, and of writing this response when the client stops receiving it.
    pub async fn serve_connection(
        &self,
        mut stream: TcpStream,
        observed_requests: &ObservedRequests,
    ) -> io::Result<()> {
        observed_requests.record(read_range_header(&mut stream).await?);

        match self {
            Self::EndlessTrickle { byte_interval } => {
                observed_requests.record_abandoned_trickle(
                    &trickle_until_the_client_stops_receiving(&mut stream, *byte_interval).await,
                );

                Ok(())
            }
            Self::HuggingFaceFile {
                body,
                commit_hash,
                etag,
            } => {
                write_complete_body(
                    &mut stream,
                    &status_line_and_headers(
                        StatusCode::PARTIAL_CONTENT,
                        body.len(),
                        &hugging_face_file_headers(commit_hash, etag),
                    ),
                    body,
                )
                .await
            }
            Self::Ok(body) => {
                write_complete_body(
                    &mut stream,
                    &status_line_and_headers(StatusCode::OK, body.len(), ""),
                    body,
                )
                .await
            }
            Self::PartialContent {
                body,
                content_range,
            } => write_partial_content(&mut stream, body.len(), content_range, body).await,
            Self::PartialContentDeclaringLength {
                content_range,
                declared_content_length,
            } => {
                write_partial_content(&mut stream, *declared_content_length, content_range, &[])
                    .await
            }
            Self::StallBeforeHeaders => pending().await,
            Self::StalledBody {
                declared_body,
                sent_byte_count,
            } => {
                write_body_prefix(
                    &mut stream,
                    StatusCode::OK,
                    "",
                    declared_body,
                    *sent_byte_count,
                )
                .await?;

                pending().await
            }
            Self::Status(status) => {
                write_complete_body(&mut stream, &status_line_and_headers(*status, 0, ""), &[])
                    .await
            }
            Self::TruncatedBody {
                declared_body,
                sent_byte_count,
                status,
            } => write_body_prefix(&mut stream, *status, "", declared_body, *sent_byte_count).await,
            Self::TruncatedHuggingFaceFile {
                commit_hash,
                declared_body,
                etag,
                sent_byte_count,
            } => {
                write_body_prefix(
                    &mut stream,
                    StatusCode::PARTIAL_CONTENT,
                    &hugging_face_file_headers(commit_hash, etag),
                    declared_body,
                    *sent_byte_count,
                )
                .await
            }
        }
    }
}
