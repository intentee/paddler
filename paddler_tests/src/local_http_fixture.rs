use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

pub struct LocalHttpFixture {
    accept_task: Option<JoinHandle<()>>,
    port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl LocalHttpFixture {
    pub async fn start(status_line: &'static str, body: Vec<u8>) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("Failed to bind 127.0.0.1:0 for LocalHttpFixture")?;
        let port = listener
            .local_addr()
            .context("LocalHttpFixture listener has no local addr")?
            .port();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let body_arc = Arc::new(body);
        let accept_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    connection = listener.accept() => {
                        let Ok((mut socket, _addr)) = connection else {
                            break;
                        };
                        let body_for_connection = body_arc.clone();
                        tokio::spawn(async move {
                            let mut buffer = [0_u8; 1024];
                            let _read = socket.read(&mut buffer).await;

                            let response = format!(
                                "{status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                body_for_connection.len()
                            );
                            let _written_headers = socket.write_all(response.as_bytes()).await;
                            let _written_body = socket.write_all(&body_for_connection).await;
                            let _flushed = socket.shutdown().await;
                        });
                    }
                }
            }
        });

        Ok(Self {
            accept_task: Some(accept_task),
            port,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for LocalHttpFixture {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(accept_task) = self.accept_task.take() {
            accept_task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::local_http_fixture::LocalHttpFixture;

    #[tokio::test]
    async fn serves_configured_status_and_body() -> Result<()> {
        let fixture = LocalHttpFixture::start("HTTP/1.1 200 OK", b"hello bytes".to_vec()).await?;
        let response = reqwest::get(fixture.url("/whatever")).await?;

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.bytes().await?.as_ref(), b"hello bytes");

        Ok(())
    }

    #[tokio::test]
    async fn serves_404_when_configured() -> Result<()> {
        let fixture = LocalHttpFixture::start("HTTP/1.1 404 Not Found", Vec::new()).await?;
        let response = reqwest::get(fixture.url("/missing")).await?;

        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn each_fixture_gets_a_distinct_port() -> Result<()> {
        let first = LocalHttpFixture::start("HTTP/1.1 200 OK", Vec::new()).await?;
        let second = LocalHttpFixture::start("HTTP/1.1 200 OK", Vec::new()).await?;

        assert_ne!(first.port(), second.port());

        Ok(())
    }

    #[tokio::test]
    async fn drop_stops_accepting_connections() -> Result<()> {
        let fixture = LocalHttpFixture::start("HTTP/1.1 200 OK", b"alive".to_vec()).await?;
        let url = fixture.url("/alive");

        let still_alive_response = reqwest::get(&url).await?;
        assert_eq!(still_alive_response.status(), reqwest::StatusCode::OK);

        drop(fixture);

        let after_drop = reqwest::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_millis(500))
            .send()
            .await;
        assert!(
            after_drop.is_err(),
            "fixture should be unreachable after drop"
        );

        Ok(())
    }
}
