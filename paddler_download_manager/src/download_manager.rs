use std::path::Path;
use std::sync::Arc;

use anyhow::anyhow;
use reqwest::Client;
use reqwest::Url;
use reqwest::header::RANGE;

use crate::download_attempt_error::DownloadAttemptError;
use crate::download_error::DownloadError;
use crate::partial_file::PartialFile;
use crate::progress_sink::ProgressSink;
use crate::response_classification::ResponseClassification;
use crate::retry_policy::RetryPolicy;
use crate::stream_to_partial_file::stream_to_partial_file;
use crate::stream_to_partial_file_error::StreamToPartialFileError;

pub struct DownloadManager {
    client: Client,
    retry_policy: RetryPolicy,
}

impl DownloadManager {
    #[must_use]
    pub fn new() -> Self {
        Self::with_policy(RetryPolicy::default())
    }

    #[must_use]
    pub fn with_policy(retry_policy: RetryPolicy) -> Self {
        Self {
            client: Client::new(),
            retry_policy,
        }
    }

    pub async fn download(
        &self,
        url: &str,
        final_path: &Path,
        progress_sink: Arc<dyn ProgressSink>,
    ) -> Result<(), DownloadError> {
        Url::parse(url).map_err(|parse_error| DownloadError::InvalidUrl {
            url: url.to_owned(),
            source: parse_error,
        })?;

        let partial = PartialFile::new(final_path.to_path_buf());
        let mut attempt: u32 = 0;

        loop {
            match self.attempt_download(url, &partial, &progress_sink).await {
                Ok(()) => return Ok(()),
                Err(DownloadAttemptError::Transient(transient_error)) => {
                    attempt += 1;

                    if attempt >= self.retry_policy.max_attempts {
                        return Err(DownloadError::NetworkExhausted {
                            url: url.to_owned(),
                            attempts: attempt,
                            source: transient_error,
                        });
                    }

                    let delay = self.retry_policy.delay_for_attempt(attempt - 1);
                    tokio::time::sleep(delay).await;
                }
                Err(DownloadAttemptError::NotFound) => {
                    return Err(DownloadError::NotFound {
                        url: url.to_owned(),
                    });
                }
                Err(DownloadAttemptError::PermissionDenied(status)) => {
                    return Err(DownloadError::PermissionDenied {
                        url: url.to_owned(),
                        status,
                    });
                }
                Err(DownloadAttemptError::PartialFileStale) => {
                    return Err(DownloadError::PartialFileStale {
                        url: url.to_owned(),
                        partial_path: partial.partial_path.clone(),
                    });
                }
                Err(DownloadAttemptError::Io(io_error)) => {
                    return Err(DownloadError::Io {
                        path: partial.partial_path.clone(),
                        source: io_error,
                    });
                }
            }
        }
    }

    async fn attempt_download(
        &self,
        url: &str,
        partial: &PartialFile,
        progress_sink: &Arc<dyn ProgressSink>,
    ) -> Result<(), DownloadAttemptError> {
        let mut offset = partial.current_size().await?;
        let sent_range_header = offset > 0;

        let mut request = self.client.get(url);
        if sent_range_header {
            request = request.header(RANGE, format!("bytes={offset}-"));
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(send_error) => {
                return Err(DownloadAttemptError::Transient(anyhow::Error::new(send_error)));
            }
        };

        let classification =
            ResponseClassification::from_status(response.status(), sent_range_header);

        match classification {
            ResponseClassification::NotFound => return Err(DownloadAttemptError::NotFound),
            ResponseClassification::PermissionDenied(status) => {
                return Err(DownloadAttemptError::PermissionDenied(status));
            }
            ResponseClassification::PartialFileStale => {
                partial.remove().await?;
                return Err(DownloadAttemptError::PartialFileStale);
            }
            ResponseClassification::Retryable(status) => {
                return Err(DownloadAttemptError::Transient(anyhow!(
                    "URL '{url}' returned {status}"
                )));
            }
            ResponseClassification::StreamFromStartIgnoringRange => {
                partial.truncate().await?;
                offset = 0;
            }
            ResponseClassification::StreamFromCurrentOffset
            | ResponseClassification::StreamFromStart => {}
        }

        let total = offset + response.content_length().unwrap_or(0);
        progress_sink.on_started(total, offset);

        let mut file = partial.open_for_append().await?;

        match stream_to_partial_file(response.bytes_stream(), &mut file, progress_sink).await {
            Ok(()) => {}
            Err(StreamToPartialFileError::Stream(stream_error)) => {
                return Err(DownloadAttemptError::Transient(anyhow::Error::new(stream_error)));
            }
            Err(StreamToPartialFileError::Write(write_error)) => {
                return Err(DownloadAttemptError::Io(write_error));
            }
        }

        drop(file);

        partial.finalize().await?;
        progress_sink.on_finished();

        Ok(())
    }
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::download_manager::DownloadManager;
    use crate::retry_policy::RetryPolicy;

    #[test]
    fn default_constructs_download_manager_with_default_retry_policy() {
        let manager = DownloadManager::default();
        let default_policy = RetryPolicy::default();

        assert_eq!(manager.retry_policy.max_attempts, default_policy.max_attempts);
        assert_eq!(
            manager.retry_policy.initial_backoff,
            default_policy.initial_backoff
        );
        assert_eq!(manager.retry_policy.max_backoff, default_policy.max_backoff);
    }
}
