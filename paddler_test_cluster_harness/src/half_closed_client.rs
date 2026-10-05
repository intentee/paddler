use std::net::SocketAddr;

use anyhow::Context as _;
use anyhow::Result;
use serde::Serialize;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpStream;

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
            .context(format!("half-closed client must reach {addr}"))?;

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

    use anyhow::Result;
    use serde_json::json;
    use tokio::io::AsyncReadExt as _;
    use tokio::net::TcpListener;

    use super::HalfClosedClient;

    #[tokio::test]
    async fn reports_the_address_it_could_not_reach() -> Result<()> {
        let unbound_listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
        let unreachable_addr = unbound_listener.local_addr()?;

        drop(unbound_listener);

        let connect_error = HalfClosedClient::post_json_then_half_close(
            unreachable_addr,
            "/api/v1/probe",
            &json!({}),
        )
        .await
        .err();

        assert!(
            connect_error
                .is_some_and(|error| error.to_string().contains(&unreachable_addr.to_string())),
            "connecting to a closed port must fail and name the address"
        );

        Ok(())
    }

    #[tokio::test]
    async fn sends_the_request_and_leaves_the_read_side_open() -> Result<()> {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
        let addr = listener.local_addr()?;

        let accepted = tokio::spawn(async move {
            let (mut accepted_socket, _peer) = listener.accept().await?;
            let mut received = Vec::new();

            accepted_socket.read_to_end(&mut received).await?;

            Ok::<Vec<u8>, anyhow::Error>(received)
        });

        let mut client =
            HalfClosedClient::post_json_then_half_close(addr, "/api/v1/probe", &json!({"a": 1}))
                .await?;

        client.half_close().await?;

        let received = String::from_utf8(accepted.await??)?;

        assert!(received.starts_with("POST /api/v1/probe HTTP/1.1\r\n"));
        assert!(received.contains("Content-Length: 7\r\n"));
        assert!(received.ends_with("{\"a\":1}"));

        Ok(())
    }
}
