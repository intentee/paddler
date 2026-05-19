use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use anyhow::Result;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::task::JoinHandle;

const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone)]
pub enum FixtureResponse {
    Ok(Vec<u8>),
    PartialContent(Vec<u8>),
    Status(u16),
    OkDropAfter {
        body: Vec<u8>,
        bytes_before_drop: usize,
    },
}

impl FixtureResponse {
    pub const fn ok(body: Vec<u8>) -> Self {
        Self::Ok(body)
    }

    pub const fn partial_content(body: Vec<u8>) -> Self {
        Self::PartialContent(body)
    }

    pub const fn status(code: u16) -> Self {
        Self::Status(code)
    }

    pub const fn ok_drop_after(body: Vec<u8>, bytes_before_drop: usize) -> Self {
        Self::OkDropAfter {
            body,
            bytes_before_drop,
        }
    }
}

pub enum Scenario {
    Always(FixtureResponse),
}

impl Scenario {
    pub const fn always(response: FixtureResponse) -> Self {
        Self::Always(response)
    }
}

pub struct LocalHttpFixture {
    accept_task: Option<JoinHandle<()>>,
    last_range_rx: watch::Receiver<Option<String>>,
    port: u16,
    request_count: Arc<AtomicU32>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl LocalHttpFixture {
    pub async fn start(scenario: Scenario) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let request_count = Arc::new(AtomicU32::new(0));
        let (last_range_tx, last_range_rx) = watch::channel(None::<String>);
        let last_range_tx = Arc::new(last_range_tx);
        let scenario_state = Arc::new(ScenarioState::from(scenario));

        let accept_request_count = request_count.clone();
        let accept_last_range_tx = last_range_tx;
        let accept_scenario_state = scenario_state;

        let accept_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accept_result = listener.accept() => {
                        let Ok((socket, _addr)) = accept_result else {
                            break;
                        };
                        let request_count_for_conn = accept_request_count.clone();
                        let last_range_tx_for_conn = accept_last_range_tx.clone();
                        let scenario_state_for_conn = accept_scenario_state.clone();

                        tokio::spawn(async move {
                            let _ = handle_connection(
                                socket,
                                request_count_for_conn,
                                last_range_tx_for_conn,
                                scenario_state_for_conn,
                            )
                            .await;
                        });
                    }
                }
            }
        });

        Ok(Self {
            accept_task: Some(accept_task),
            last_range_rx,
            port,
            request_count,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    pub fn request_count(&self) -> u32 {
        self.request_count.load(Ordering::Relaxed)
    }

    pub fn last_recorded_range_header(&self) -> Option<String> {
        self.last_range_rx.borrow().clone()
    }
}

impl Drop for LocalHttpFixture {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        if let Some(task) = self.accept_task.take() {
            task.abort();
        }
    }
}

enum ScenarioState {
    Always(FixtureResponse),
}

impl ScenarioState {
    fn next(&self) -> FixtureResponse {
        match self {
            Self::Always(response) => response.clone(),
        }
    }
}

impl From<Scenario> for ScenarioState {
    fn from(scenario: Scenario) -> Self {
        match scenario {
            Scenario::Always(response) => Self::Always(response),
        }
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    request_count: Arc<AtomicU32>,
    last_range_tx: Arc<watch::Sender<Option<String>>>,
    scenario_state: Arc<ScenarioState>,
) -> Result<()> {
    let (reader_half, mut writer_half) = socket.split();
    let mut reader = BufReader::new(reader_half);

    let mut request_line = String::new();
    tokio::time::timeout(READ_TIMEOUT, reader.read_line(&mut request_line)).await??;

    let mut range_header_value: Option<String> = None;
    loop {
        let mut header_line = String::new();
        let bytes_read =
            tokio::time::timeout(READ_TIMEOUT, reader.read_line(&mut header_line)).await??;
        if bytes_read == 0 || header_line == "\r\n" || header_line == "\n" {
            break;
        }
        if let Some(rest) = header_line.strip_prefix("Range:") {
            range_header_value = Some(rest.trim().to_owned());
        } else if let Some(rest) = header_line.strip_prefix("range:") {
            range_header_value = Some(rest.trim().to_owned());
        }
    }

    last_range_tx.send_replace(range_header_value);
    request_count.fetch_add(1, Ordering::Relaxed);

    let response = scenario_state.next();
    write_response(&mut writer_half, response).await?;
    Ok(())
}

async fn write_response<TWriter>(writer: &mut TWriter, response: FixtureResponse) -> io::Result<()>
where
    TWriter: AsyncWriteExt + Unpin,
{
    match response {
        FixtureResponse::Ok(body) => {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            writer.write_all(header.as_bytes()).await?;
            writer.write_all(&body).await?;
            writer.shutdown().await?;
        }
        FixtureResponse::PartialContent(body) => {
            let header = format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            writer.write_all(header.as_bytes()).await?;
            writer.write_all(&body).await?;
            writer.shutdown().await?;
        }
        FixtureResponse::Status(code) => {
            let status_text = match code {
                401 => "Unauthorized",
                403 => "Forbidden",
                404 => "Not Found",
                416 => "Range Not Satisfiable",
                500 => "Internal Server Error",
                503 => "Service Unavailable",
                _ => "Other",
            };
            let header = format!(
                "HTTP/1.1 {code} {status_text}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            writer.write_all(header.as_bytes()).await?;
            writer.shutdown().await?;
        }
        FixtureResponse::OkDropAfter {
            body,
            bytes_before_drop,
        } => {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            writer.write_all(header.as_bytes()).await?;
            let truncated_len = bytes_before_drop.min(body.len());
            writer.write_all(&body[..truncated_len]).await?;
        }
    }
    Ok(())
}
