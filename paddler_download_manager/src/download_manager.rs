use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use headers::ContentRange;
use headers::HeaderMapExt as _;
use reqwest::Client;
use reqwest::Url;
use reqwest::header::RANGE;

use crate::download_attempt_error::DownloadAttemptError;
use crate::download_error::DownloadError;
use crate::partial_file::PartialFile;
use crate::progress_sink::ProgressSink;
use crate::response_classification::ResponseClassification;
use crate::stream_to_partial_file::stream_to_partial_file;
use crate::stream_to_partial_file_error::StreamToPartialFileError;

#[cfg(unix)]
fn is_disk_full(error: &io::Error) -> bool {
    error.raw_os_error() == Some(28)
}

#[cfg(windows)]
fn is_disk_full(error: &io::Error) -> bool {
    error.raw_os_error() == Some(112)
}

fn classify_cache_failure(path: PathBuf, source: io::Error) -> DownloadError {
    if source.kind() == io::ErrorKind::PermissionDenied {
        DownloadError::CachePermissionDenied { path, source }
    } else if is_disk_full(&source) {
        DownloadError::CacheDiskFull { path, source }
    } else {
        DownloadError::Io { path, source }
    }
}

pub struct DownloadManager {
    client: Client,
}

impl DownloadManager {
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self { client })
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

        match self.attempt_download(url, &partial, &progress_sink).await {
            Ok(()) => Ok(()),
            Err(DownloadAttemptError::Unreachable(source)) => {
                Err(DownloadError::DownloadServerIsUnreachable {
                    url: url.to_owned(),
                    source,
                })
            }
            Err(DownloadAttemptError::ServerError(status)) => {
                Err(DownloadError::DownloadServerErrored {
                    url: url.to_owned(),
                    status,
                })
            }
            Err(DownloadAttemptError::Interrupted(source)) => {
                Err(DownloadError::DownloadInterrupted {
                    url: url.to_owned(),
                    source,
                })
            }
            Err(DownloadAttemptError::NotFound) => Err(DownloadError::NotFound {
                url: url.to_owned(),
            }),
            Err(DownloadAttemptError::PermissionDenied(status)) => {
                Err(DownloadError::PermissionDenied {
                    url: url.to_owned(),
                    status,
                })
            }
            Err(DownloadAttemptError::PartialFileStale) => Err(DownloadError::PartialFileStale {
                url: url.to_owned(),
                partial_path: partial.partial_path.clone(),
            }),
            Err(DownloadAttemptError::Io(io_error)) => Err(classify_cache_failure(
                partial.partial_path.clone(),
                io_error,
            )),
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
                return Err(DownloadAttemptError::Unreachable(anyhow::Error::new(send_error)));
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
            ResponseClassification::ServerError(status) => {
                return Err(DownloadAttemptError::ServerError(status));
            }
            ResponseClassification::StreamFromStartIgnoringRange => {
                partial.truncate().await?;
                offset = 0;
            }
            ResponseClassification::StreamFromCurrentOffset
            | ResponseClassification::StreamFromStart => {}
        }

        if matches!(classification, ResponseClassification::StreamFromCurrentOffset) {
            let server_start = response
                .headers()
                .typed_get::<ContentRange>()
                .and_then(|content_range| content_range.bytes_range())
                .map(|(start, _end)| start);

            if server_start != Some(offset) {
                partial.remove().await?;

                return Err(DownloadAttemptError::PartialFileStale);
            }
        }

        let total = response
            .content_length()
            .map(|content_length| offset + content_length);
        progress_sink.on_started(total, offset);

        let mut file = partial.open_for_append().await?;

        match stream_to_partial_file(response.bytes_stream(), &mut file, progress_sink).await {
            Ok(()) => {}
            Err(StreamToPartialFileError::Stream(stream_error)) => {
                return Err(DownloadAttemptError::Interrupted(anyhow::Error::new(
                    stream_error,
                )));
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


