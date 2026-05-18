use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use futures_util::StreamExt;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::ModelPath;
use paddler_types::url_model_reference::UrlModelReference;
use sha2::Digest;
use sha2::Sha256;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::agent_issue_fix::AgentIssueFix;
use crate::desired_model_resolution::DesiredModelResolution;
use crate::paddler_cache_dir::PaddlerCacheDir;
use crate::resolves_model_source::ResolvesModelSource;
use crate::slot_aggregated_status::SlotAggregatedStatus;

const DEFAULT_BASENAME: &str = "model.gguf";

fn hex_lowercase(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut acc, byte| {
            let _ = write!(acc, "{byte:02x}");
            acc
        })
}

fn url_basename(parsed: &Url) -> String {
    parsed
        .path_segments()
        .and_then(|mut segments| {
            segments
                .rfind(|segment| !segment.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| DEFAULT_BASENAME.to_owned())
}

fn url_cache_path(cache_root: &Path, url_string: &str, parsed: &Url) -> PathBuf {
    let digest = Sha256::digest(url_string.as_bytes());
    let hex_digest = hex_lowercase(&digest);
    let basename = url_basename(parsed);

    cache_root
        .join("url-models")
        .join(hex_digest)
        .join(basename)
}

fn content_length_as_usize(response: &reqwest::Response) -> Result<usize> {
    response.content_length().map_or(Ok(0), |length| {
        usize::try_from(length).map_err(|conversion_error| {
            anyhow!("Content-Length '{length}' does not fit in usize: {conversion_error}")
        })
    })
}

async fn write_response_to_partial_file(
    response: reqwest::Response,
    partial_path: &Path,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
) -> Result<()> {
    let mut file = fs::File::create(partial_path).await.with_context(|| {
        format!(
            "Failed to create partial download file '{}'",
            partial_path.display()
        )
    })?;
    let mut stream = response.bytes_stream();

    while let Some(next_chunk) = stream.next().await {
        let bytes = next_chunk.context("Stream error while downloading model bytes")?;

        file.write_all(&bytes).await.with_context(|| {
            format!(
                "Failed to write chunk to partial file '{}'",
                partial_path.display()
            )
        })?;
        slot_aggregated_status.increment_download_current(bytes.len());
    }

    file.flush().await?;

    Ok(())
}

async fn download_url_model(
    url_string: &str,
    cache_path: &Path,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
) -> Result<()> {
    let parent = cache_path
        .parent()
        .with_context(|| format!("Cache path '{}' has no parent", cache_path.display()))?;

    fs::create_dir_all(parent)
        .await
        .with_context(|| format!("Failed to create cache directory '{}'", parent.display()))?;

    let response = match reqwest::Client::new().get(url_string).send().await {
        Ok(response) => response,
        Err(send_error) => {
            slot_aggregated_status.register_issue(AgentIssue::UrlModelDownloadFailed(ModelPath {
                model_path: url_string.to_owned(),
            }));

            return Err(send_error).with_context(|| format!("Failed to GET '{url_string}'"));
        }
    };

    let status = response.status();

    if status == reqwest::StatusCode::NOT_FOUND {
        slot_aggregated_status.register_issue(AgentIssue::UrlModelNotFound(ModelPath {
            model_path: url_string.to_owned(),
        }));

        return Err(anyhow!("Model URL '{url_string}' returned 404 Not Found"));
    }

    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        slot_aggregated_status.register_issue(AgentIssue::UrlModelPermissionDenied(ModelPath {
            model_path: url_string.to_owned(),
        }));

        return Err(anyhow!("Model URL '{url_string}' returned {status}"));
    }

    if !status.is_success() {
        slot_aggregated_status.register_issue(AgentIssue::UrlModelDownloadFailed(ModelPath {
            model_path: url_string.to_owned(),
        }));

        return Err(anyhow!("Model URL '{url_string}' returned {status}"));
    }

    let total = content_length_as_usize(&response)?;
    let basename = cache_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned);

    slot_aggregated_status.set_download_status(0, total, basename);
    slot_aggregated_status.register_fix(&AgentIssueFix::UrlModelStartedDownloading(ModelPath {
        model_path: url_string.to_owned(),
    }));

    let partial_path = cache_path.with_extension("partial");

    write_response_to_partial_file(response, &partial_path, slot_aggregated_status.clone()).await?;

    fs::rename(&partial_path, cache_path)
        .await
        .with_context(|| {
            format!(
                "Failed to rename '{}' -> '{}'",
                partial_path.display(),
                cache_path.display()
            )
        })?;

    slot_aggregated_status.register_fix(&AgentIssueFix::UrlModelDownloaded(ModelPath {
        model_path: url_string.to_owned(),
    }));
    slot_aggregated_status.reset_download();

    Ok(())
}

async fn resolve_url_into_cache(
    url_string: &str,
    cache_root: &Path,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
) -> Result<DesiredModelResolution> {
    let parsed = Url::parse(url_string).with_context(|| format!("Invalid URL '{url_string}'"))?;
    let cache_path = url_cache_path(cache_root, url_string, &parsed);

    if fs::try_exists(&cache_path).await? {
        slot_aggregated_status.reset_download();

        return Ok(DesiredModelResolution::Resolved(cache_path));
    }

    download_url_model(url_string, &cache_path, slot_aggregated_status).await?;

    Ok(DesiredModelResolution::Resolved(cache_path))
}

#[async_trait]
impl ResolvesModelSource for UrlModelReference {
    async fn resolve(
        &self,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution> {
        let cache_root = PaddlerCacheDir::from_process_env().resolve()?;

        resolve_url_into_cache(&self.url, &cache_root, slot_aggregated_status).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use anyhow::anyhow;
    use paddler_types::agent_issue::AgentIssue;
    use paddler_types::agent_issue_params::ModelPath;
    use sha2::Digest;
    use sha2::Sha256;
    use tempfile::TempDir;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use url::Url;

    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::model_source::url::resolve_url_into_cache;
    use crate::model_source::url::url_basename;
    use crate::model_source::url::url_cache_path;
    use crate::slot_aggregated_status::SlotAggregatedStatus;

    struct FixtureServer {
        port: u16,
        _shutdown: oneshot::Sender<()>,
    }

    impl FixtureServer {
        async fn start(status_line: &'static str, body: Vec<u8>) -> Result<Self> {
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let port = listener.local_addr()?.port();
            let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
            let body_arc = Arc::new(body);

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = &mut shutdown_rx => break,
                        connection = listener.accept() => {
                            if let Ok((mut socket, _addr)) = connection {
                                let body = body_arc.clone();
                                tokio::spawn(async move {
                                    let mut buffer = [0_u8; 1024];
                                    let _ = socket.read(&mut buffer).await;

                                    let response = format!(
                                        "{status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                        body.len()
                                    );
                                    let _ = socket.write_all(response.as_bytes()).await;
                                    let _ = socket.write_all(&body).await;
                                    let _ = socket.shutdown().await;
                                });
                            }
                        }
                    }
                }
            });

            Ok(Self {
                port,
                _shutdown: shutdown_tx,
            })
        }

        fn url(&self, path: &str) -> String {
            format!("http://127.0.0.1:{}{path}", self.port)
        }
    }

    fn fresh_status() -> Arc<SlotAggregatedStatus> {
        Arc::new(SlotAggregatedStatus::new(1))
    }

    #[test]
    fn basename_uses_last_path_segment() -> Result<()> {
        let parsed = Url::parse("https://host.example/folder/model.gguf")?;

        assert_eq!(url_basename(&parsed), "model.gguf");

        Ok(())
    }

    #[test]
    fn basename_falls_back_to_model_gguf_when_path_empty() -> Result<()> {
        let parsed = Url::parse("https://host.example/")?;

        assert_eq!(url_basename(&parsed), "model.gguf");

        Ok(())
    }

    #[test]
    fn basename_ignores_trailing_slash() -> Result<()> {
        let parsed = Url::parse("https://host.example/folder/model.gguf/")?;

        assert_eq!(url_basename(&parsed), "model.gguf");

        Ok(())
    }

    #[test]
    fn cache_path_is_sha256_of_url_with_basename() -> Result<()> {
        let cache_root = TempDir::new()?;
        let url_string = "https://host.example/folder/model.gguf";
        let parsed = Url::parse(url_string)?;

        let path = url_cache_path(cache_root.path(), url_string, &parsed);
        let path_string = path.to_string_lossy().into_owned();
        let expected_hex = super::hex_lowercase(&Sha256::digest(url_string.as_bytes()));

        assert!(path_string.contains("url-models"));
        assert!(path_string.ends_with("/model.gguf"));
        assert!(path_string.contains(&expected_hex));

        Ok(())
    }

    #[tokio::test]
    async fn download_succeeds_against_fixture_server_and_writes_bytes() -> Result<()> {
        let cache_root = TempDir::new()?;
        let body = b"GGUF placeholder bytes".to_vec();
        let server = FixtureServer::start("HTTP/1.1 200 OK", body.clone()).await?;
        let status = fresh_status();
        let url_string = server.url("/model.gguf");

        let resolution = resolve_url_into_cache(&url_string, cache_root.path(), status).await?;

        let path = match resolution {
            DesiredModelResolution::Resolved(path) => path,
            other => return Err(anyhow!("expected Resolved, got {other:?}")),
        };
        let mut content = Vec::new();
        tokio::fs::File::open(&path)
            .await?
            .read_to_end(&mut content)
            .await?;

        assert_eq!(content, body);

        Ok(())
    }

    #[tokio::test]
    async fn download_404_returns_error_and_registers_url_model_not_found() -> Result<()> {
        let cache_root = TempDir::new()?;
        let server = FixtureServer::start("HTTP/1.1 404 Not Found", Vec::new()).await?;
        let status = fresh_status();
        let url_string = server.url("/missing.gguf");

        let result = resolve_url_into_cache(&url_string, cache_root.path(), status.clone()).await;

        assert!(result.is_err());
        assert!(status.has_issue(&AgentIssue::UrlModelNotFound(ModelPath {
            model_path: url_string,
        })));

        Ok(())
    }

    #[tokio::test]
    async fn download_401_returns_error_and_registers_url_model_permission_denied() -> Result<()> {
        let cache_root = TempDir::new()?;
        let server = FixtureServer::start("HTTP/1.1 401 Unauthorized", Vec::new()).await?;
        let status = fresh_status();
        let url_string = server.url("/private.gguf");

        let result = resolve_url_into_cache(&url_string, cache_root.path(), status.clone()).await;

        assert!(result.is_err());
        assert!(
            status.has_issue(&AgentIssue::UrlModelPermissionDenied(ModelPath {
                model_path: url_string,
            }))
        );

        Ok(())
    }

    #[tokio::test]
    async fn download_403_returns_error_and_registers_url_model_permission_denied() -> Result<()> {
        let cache_root = TempDir::new()?;
        let server = FixtureServer::start("HTTP/1.1 403 Forbidden", Vec::new()).await?;
        let status = fresh_status();
        let url_string = server.url("/forbidden.gguf");

        let result = resolve_url_into_cache(&url_string, cache_root.path(), status.clone()).await;

        assert!(result.is_err());
        assert!(
            status.has_issue(&AgentIssue::UrlModelPermissionDenied(ModelPath {
                model_path: url_string,
            }))
        );

        Ok(())
    }

    #[tokio::test]
    async fn download_500_returns_error_and_registers_url_model_download_failed() -> Result<()> {
        let cache_root = TempDir::new()?;
        let server = FixtureServer::start("HTTP/1.1 500 Internal Server Error", Vec::new()).await?;
        let status = fresh_status();
        let url_string = server.url("/broken.gguf");

        let result = resolve_url_into_cache(&url_string, cache_root.path(), status.clone()).await;

        assert!(result.is_err());
        assert!(
            status.has_issue(&AgentIssue::UrlModelDownloadFailed(ModelPath {
                model_path: url_string,
            }))
        );

        Ok(())
    }

    #[tokio::test]
    async fn cache_hit_returns_path_without_http_request() -> Result<()> {
        let cache_root = TempDir::new()?;
        // Server returns 500 so any real call would fail. The cache hit must avoid the call.
        let server = FixtureServer::start("HTTP/1.1 500 Internal Server Error", Vec::new()).await?;
        let status = fresh_status();
        let url_string = server.url("/cached.gguf");
        let parsed = Url::parse(&url_string)?;
        let expected = url_cache_path(cache_root.path(), &url_string, &parsed);
        if let Some(parent) = expected.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&expected, b"cached content").await?;

        let resolution = resolve_url_into_cache(&url_string, cache_root.path(), status).await?;

        match resolution {
            DesiredModelResolution::Resolved(path) => assert_eq!(path, expected),
            other => return Err(anyhow!("expected Resolved, got {other:?}")),
        }

        Ok(())
    }
}
