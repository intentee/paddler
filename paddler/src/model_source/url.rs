use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use paddler_cache_dir::CacheDir;
use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::progress_sink::ProgressSink;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::ModelPath;
use paddler_types::url_model_reference::UrlModelReference;
use sha2::Digest;
use sha2::Sha256;
use tokio::fs;
use url::Url;

use crate::agent_issue_fix::AgentIssueFix;
use crate::desired_model_resolution::DesiredModelResolution;
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
        .join("downloaded-models")
        .join(hex_digest)
        .join(basename)
}

struct SlotAggregatedStatusSink {
    basename: Option<String>,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
    url: String,
}

impl ProgressSink for SlotAggregatedStatusSink {
    fn on_started(&self, total_bytes: u64, already_downloaded: u64) {
        let total = usize::try_from(total_bytes).unwrap_or(usize::MAX);
        let current = usize::try_from(already_downloaded).unwrap_or(usize::MAX);

        self.slot_aggregated_status
            .set_download_status(current, total, self.basename.clone());
        self.slot_aggregated_status
            .register_fix(&AgentIssueFix::UrlModelStartedDownloading(ModelPath {
                model_path: self.url.clone(),
            }));
    }

    fn on_chunk(&self, additional_bytes: u64) {
        let bytes = usize::try_from(additional_bytes).unwrap_or(usize::MAX);

        self.slot_aggregated_status
            .increment_download_current(bytes);
    }

    fn on_finished(&self) {
        self.slot_aggregated_status
            .register_fix(&AgentIssueFix::UrlModelDownloaded(ModelPath {
                model_path: self.url.clone(),
            }));
        self.slot_aggregated_status.reset_download();
    }
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

    let basename = cache_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned);
    let sink: Arc<dyn ProgressSink> = Arc::new(SlotAggregatedStatusSink {
        basename,
        slot_aggregated_status: slot_aggregated_status.clone(),
        url: url_string.to_owned(),
    });

    match DownloadManager::new()
        .download(url_string, &cache_path, sink)
        .await
    {
        Ok(()) => Ok(DesiredModelResolution::Resolved(cache_path)),
        Err(DownloadError::NotFound { url }) => {
            slot_aggregated_status.register_issue(AgentIssue::UrlModelNotFound(ModelPath {
                model_path: url.clone(),
            }));

            Err(anyhow!("Model URL '{url}' returned 404 Not Found"))
        }
        Err(DownloadError::PermissionDenied { url, status }) => {
            slot_aggregated_status.register_issue(AgentIssue::UrlModelPermissionDenied(
                ModelPath {
                    model_path: url.clone(),
                },
            ));

            Err(anyhow!("Model URL '{url}' returned {status}"))
        }
        Err(other) => {
            let url_for_issue = url_string.to_owned();
            slot_aggregated_status.register_issue(AgentIssue::UrlModelDownloadFailed(ModelPath {
                model_path: url_for_issue,
            }));

            Err(anyhow::Error::new(other))
        }
    }
}

#[async_trait]
impl ResolvesModelSource for UrlModelReference {
    async fn resolve(
        &self,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution> {
        let cache_root = CacheDir::from_process_env().resolve()?;

        resolve_url_into_cache(&self.url, &cache_root, slot_aggregated_status).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use anyhow::anyhow;
    use sha2::Digest;
    use sha2::Sha256;
    use tempfile::TempDir;
    use url::Url;

    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::model_source::url::resolve_url_into_cache;
    use crate::model_source::url::url_basename;
    use crate::model_source::url::url_cache_path;
    use crate::slot_aggregated_status::SlotAggregatedStatus;

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

        assert!(path_string.contains("downloaded-models"));
        assert!(path_string.ends_with("/model.gguf"));
        assert!(path_string.contains(&expected_hex));

        Ok(())
    }

    #[tokio::test]
    async fn cache_hit_returns_path_without_calling_download_manager() -> Result<()> {
        let cache_root = TempDir::new()?;
        let url_string = "https://host.example/cached.gguf";
        let parsed = Url::parse(url_string)?;
        let expected = url_cache_path(cache_root.path(), url_string, &parsed);
        if let Some(parent) = expected.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&expected, b"cached content").await?;

        let resolution =
            resolve_url_into_cache(url_string, cache_root.path(), fresh_status()).await?;

        match resolution {
            DesiredModelResolution::Resolved(path) => assert_eq!(path, expected),
            other => return Err(anyhow!("expected Resolved, got {other:?}")),
        }

        Ok(())
    }
}
