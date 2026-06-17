use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use tokio::io::AsyncReadExt as _;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::http_response_spec::HttpResponseSpec;

fn render_response(response: &HttpResponseSpec) -> Vec<u8> {
    let mut rendered = Vec::new();

    rendered.extend_from_slice(response.status_line.as_bytes());
    rendered.extend_from_slice(b"\r\n");

    for header in &response.headers {
        rendered.extend_from_slice(header.name.as_bytes());
        rendered.extend_from_slice(b": ");
        rendered.extend_from_slice(&header.value);
        rendered.extend_from_slice(b"\r\n");
    }

    let declared_content_length = response.body.len() + response.phantom_content_length_bytes;

    rendered.extend_from_slice(format!("Content-Length: {declared_content_length}\r\n").as_bytes());
    rendered.extend_from_slice(b"Connection: close\r\n\r\n");
    rendered.extend_from_slice(&response.body);

    rendered
}

pub struct LocalHttpFixture {
    accept_task: Option<JoinHandle<()>>,
    port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl LocalHttpFixture {
    pub async fn start(response: HttpResponseSpec) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind a local fixture listener")?;
        let port = listener
            .local_addr()
            .context("local fixture listener has no local address")?
            .port();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let rendered_response = Arc::new(render_response(&response));

        let accept_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accepted = listener.accept() => {
                        let Ok((mut socket, _address)) = accepted else {
                            break;
                        };
                        let response_for_connection = rendered_response.clone();

                        tokio::spawn(async move {
                            let mut request_buffer = [0_u8; 1024];
                            let _read = socket.read(&mut request_buffer).await;
                            let _written = socket.write_all(&response_for_connection).await;
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
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
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
