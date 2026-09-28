use std::net::SocketAddr;

use anyhow::Context as _;
use anyhow::Result;
use serde::Serialize;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpStream;

use crate::cluster_harness_error::ClusterHarnessError;

pub struct HalfClosedClient {
    socket: TcpStream,
}

impl HalfClosedClient {
    pub async fn post_json_then_half_close<TBody>(
        addr: SocketAddr,
        path: &str,
        body: &TBody,
    ) -> Result<Self>
    where
        TBody: Serialize,
    {
        let serialized_body = serde_json::to_string(body)?;
        let request = format!(
            "POST {path} HTTP/1.1\r\n\
             Host: {addr}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {content_length}\r\n\
             \r\n\
             {serialized_body}",
            content_length = serialized_body.len(),
        );

        let mut socket = TcpStream::connect(addr)
            .await
            .map_err(|source| ClusterHarnessError::HalfClosedClientUnreachable { addr, source })?;

        socket.write_all(request.as_bytes()).await?;
        socket.flush().await?;

        Ok(Self { socket })
    }

    pub async fn half_close(&mut self) -> Result<()> {
        self.socket
            .shutdown()
            .await
            .context("half-closed client must shut down only its write side")
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use serde_json::json;
    use tokio::io::AsyncReadExt as _;
    use tokio::net::TcpListener;

    use super::HalfClosedClient;
    use crate::cluster_harness_error::ClusterHarnessError;

    #[tokio::test]
    async fn reports_the_address_it_could_not_reach() {
        let unreachable_addr = SocketAddr::from(([127, 0, 0, 1], 1));

        let connect_error = HalfClosedClient::post_json_then_half_close(
            unreachable_addr,
            "/api/v1/probe",
            &json!({}),
        )
        .await
        .err()
        .expect("connecting to a closed port must fail");

        assert!(matches!(
            connect_error.downcast_ref::<ClusterHarnessError>(),
            Some(ClusterHarnessError::HalfClosedClientUnreachable { addr, .. }) if *addr == unreachable_addr
        ));
    }

    #[tokio::test]
    async fn sends_the_request_and_leaves_the_read_side_open() {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .expect("the receiving socket must bind");
        let addr = listener
            .local_addr()
            .expect("the receiving socket must report its address");
        let accepted = tokio::spawn(async move {
            let (mut accepted_socket, _peer) = listener
                .accept()
                .await
                .expect("the receiving socket must accept the client");
            let mut received = Vec::new();

            accepted_socket
                .read_to_end(&mut received)
                .await
                .expect("the request must be readable until the client half-closes");

            received
        });

        let mut client =
            HalfClosedClient::post_json_then_half_close(addr, "/api/v1/probe", &json!({"a": 1}))
                .await
                .expect("the client must send its request");

        client
            .half_close()
            .await
            .expect("the client must half-close its write side");

        let received =
            String::from_utf8(accepted.await.expect("the receiving task must not panic"))
                .expect("the request must be text");

        assert!(received.starts_with("POST /api/v1/probe HTTP/1.1\r\n"));
        assert!(received.contains("Content-Length: 7\r\n"));
        assert!(received.ends_with("{\"a\":1}"));
    }
}
