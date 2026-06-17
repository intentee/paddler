use anyhow::Context as _;
use anyhow::Result;
use futures_util::SinkExt as _;
use futures_util::StreamExt as _;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_tungstenite::accept_async;

use crate::web_socket_behavior::WebSocketBehavior;

async fn serve_connection(stream: TcpStream, behavior: WebSocketBehavior) {
    let Ok(websocket) = accept_async(stream).await else {
        return;
    };

    let (mut write, mut read) = websocket.split();

    match behavior {
        WebSocketBehavior::CloseAfterAccept => {
            let _first_request = read.next().await;
            let _ = write.close().await;
        }
        WebSocketBehavior::KeepOpen => {
            while read.next().await.is_some() {}
        }
    }
}

pub struct LocalWebSocketFixture {
    accept_task: Option<JoinHandle<()>>,
    port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl LocalWebSocketFixture {
    pub async fn start(behavior: WebSocketBehavior) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind a local websocket fixture listener")?;
        let port = listener
            .local_addr()
            .context("local websocket fixture listener has no local address")?
            .port();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let accept_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accepted = listener.accept() => {
                        let Ok((stream, _address)) = accepted else {
                            break;
                        };

                        tokio::spawn(serve_connection(stream, behavior));
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

impl Drop for LocalWebSocketFixture {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(accept_task) = self.accept_task.take() {
            accept_task.abort();
        }
    }
}
