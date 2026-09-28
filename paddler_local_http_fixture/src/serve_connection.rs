use std::future::pending;
use std::io;

use http::StatusCode;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpStream;

use crate::fixture_response::FixtureResponse;
use crate::observed_requests::ObservedRequests;
use crate::read_range_header::read_range_header;

fn status_line_and_headers(
    status: StatusCode,
    content_length: usize,
    extra_headers: &str,
) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {content_length}\r\n{extra_headers}Connection: close\r\n\r\n"
    )
}

async fn write_complete_body(stream: &mut TcpStream, head: &str, body: &[u8]) -> io::Result<()> {
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.shutdown().await
}

async fn write_body_prefix(
    stream: &mut TcpStream,
    sent_body: &[u8],
    withheld_byte_count: usize,
) -> io::Result<()> {
    stream
        .write_all(
            status_line_and_headers(StatusCode::OK, sent_body.len() + withheld_byte_count, "")
                .as_bytes(),
        )
        .await?;
    stream.write_all(sent_body).await
}

pub async fn serve_connection(
    mut stream: TcpStream,
    response: &FixtureResponse,
    observed_requests: &ObservedRequests,
) -> io::Result<()> {
    observed_requests.record(read_range_header(&mut stream).await?);

    match response {
        FixtureResponse::Ok(body) => {
            write_complete_body(
                &mut stream,
                &status_line_and_headers(StatusCode::OK, body.len(), ""),
                body,
            )
            .await
        }
        FixtureResponse::PartialContent {
            body,
            content_range,
        } => {
            write_complete_body(
                &mut stream,
                &status_line_and_headers(
                    StatusCode::PARTIAL_CONTENT,
                    body.len(),
                    &format!("Content-Range: {content_range}\r\n"),
                ),
                body,
            )
            .await
        }
        FixtureResponse::StallBeforeHeaders => pending().await,
        FixtureResponse::StalledBody {
            sent_body,
            withheld_byte_count,
        } => {
            write_body_prefix(&mut stream, sent_body, *withheld_byte_count).await?;
            stream.flush().await?;

            pending().await
        }
        FixtureResponse::Status(status) => {
            write_complete_body(&mut stream, &status_line_and_headers(*status, 0, ""), &[]).await
        }
        FixtureResponse::TruncatedBody {
            sent_body,
            withheld_byte_count,
        } => write_body_prefix(&mut stream, sent_body, *withheld_byte_count).await,
    }
}
