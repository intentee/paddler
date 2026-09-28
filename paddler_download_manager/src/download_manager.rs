use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use bytes::Bytes;
use futures_util::Stream;
use futures_util::StreamExt as _;
use headers::ContentRange;
use headers::HeaderMapExt as _;
use reqwest::Client;
use reqwest::Response;
use reqwest::Url;
use reqwest::header::RANGE;
use tokio::fs::File;
use tokio::io::AsyncWriteExt as _;
use tokio_util::sync::CancellationToken;

use crate::download_error::DownloadError;
use crate::download_outcome::DownloadOutcome;
use crate::download_progress::DownloadProgress;
use crate::partial_file::PartialFile;
use crate::response_classification::ResponseClassification;

fn classify_cache_failure(path: PathBuf, source: io::Error) -> DownloadError {
    match source.kind() {
        io::ErrorKind::PermissionDenied => DownloadError::CachePermissionDenied { path, source },
        io::ErrorKind::StorageFull => DownloadError::CacheDiskFull { path, source },
        _ => DownloadError::Io { path, source },
    }
}

struct PartialFileDownload<'download, TProgressHandler> {
    cancellation_token: &'download CancellationToken,
    on_progress: &'download TProgressHandler,
    partial: PartialFile,
    url: &'download str,
}

impl<TProgressHandler> PartialFileDownload<'_, TProgressHandler>
where
    TProgressHandler: Fn(DownloadProgress) + Sync,
{
    fn cache_failure(&self, source: io::Error) -> DownloadError {
        classify_cache_failure(self.partial.partial_path.clone(), source)
    }

    async fn remove_stale_partial(&self) -> Result<DownloadOutcome, DownloadError> {
        self.partial
            .remove()
            .await
            .map_err(|source| self.cache_failure(source))?;

        Err(DownloadError::PartialFileStale {
            url: self.url.to_owned(),
            partial_path: self.partial.partial_path.clone(),
        })
    }

    async fn run(self, client: &Client) -> Result<DownloadOutcome, DownloadError> {
        let offset = self
            .partial
            .current_size()
            .await
            .map_err(|source| self.cache_failure(source))?;
        let sent_range_header = offset > 0;
        let mut request = client.get(self.url);

        if sent_range_header {
            request = request.header(RANGE, format!("bytes={offset}-"));
        }

        let response = match self
            .cancellation_token
            .run_until_cancelled(request.send())
            .await
        {
            None => return Ok(DownloadOutcome::Cancelled),
            Some(send_result) => {
                send_result.map_err(|send_error| DownloadError::DownloadServerIsUnreachable {
                    url: self.url.to_owned(),
                    source: anyhow::Error::new(send_error),
                })?
            }
        };

        match ResponseClassification::from_status(response.status(), sent_range_header) {
            ResponseClassification::NotFound => Err(DownloadError::NotFound {
                url: self.url.to_owned(),
            }),
            ResponseClassification::PermissionDenied(status) => {
                Err(DownloadError::PermissionDenied {
                    url: self.url.to_owned(),
                    status,
                })
            }
            ResponseClassification::PartialFileStale => self.remove_stale_partial().await,
            ResponseClassification::ServerError(status) => {
                Err(DownloadError::DownloadServerErrored {
                    url: self.url.to_owned(),
                    status,
                })
            }
            ResponseClassification::ClientError(status) => {
                Err(DownloadError::DownloadServerRejectedRequest {
                    url: self.url.to_owned(),
                    status,
                })
            }
            ResponseClassification::StreamFromStartIgnoringRange => {
                self.partial
                    .truncate()
                    .await
                    .map_err(|source| self.cache_failure(source))?;
                self.stream_response(response, 0).await
            }
            ResponseClassification::StreamFromCurrentOffset => {
                let server_start = response
                    .headers()
                    .typed_get::<ContentRange>()
                    .and_then(|content_range| content_range.bytes_range())
                    .map(|(start, _end)| start);

                if server_start == Some(offset) {
                    self.stream_response(response, offset).await
                } else {
                    self.remove_stale_partial().await
                }
            }
            ResponseClassification::StreamFromStart => self.stream_response(response, offset).await,
        }
    }

    async fn stream_response(
        self,
        response: Response,
        offset: u64,
    ) -> Result<DownloadOutcome, DownloadError> {
        (self.on_progress)(DownloadProgress::Started {
            already_downloaded_bytes: offset,
            total_bytes: response
                .content_length()
                .map(|content_length| offset + content_length),
        });

        let mut file = self
            .partial
            .open_for_append()
            .await
            .map_err(|source| self.cache_failure(source))?;
        let stream_outcome = self
            .write_body_stream(response.bytes_stream(), &mut file)
            .await?;

        drop(file);

        match stream_outcome {
            DownloadOutcome::Cancelled => Ok(DownloadOutcome::Cancelled),
            DownloadOutcome::Completed => {
                self.partial
                    .finalize()
                    .await
                    .map_err(|source| self.cache_failure(source))?;
                (self.on_progress)(DownloadProgress::Finished);

                Ok(DownloadOutcome::Completed)
            }
        }
    }

    async fn write_body_stream<TStream>(
        &self,
        mut body_stream: TStream,
        file: &mut File,
    ) -> Result<DownloadOutcome, DownloadError>
    where
        TStream: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
    {
        let outcome = loop {
            match self
                .cancellation_token
                .run_until_cancelled(body_stream.next())
                .await
            {
                None => break DownloadOutcome::Cancelled,
                Some(None) => break DownloadOutcome::Completed,
                Some(Some(next_chunk)) => {
                    let chunk =
                        next_chunk.map_err(|stream_error| DownloadError::DownloadInterrupted {
                            url: self.url.to_owned(),
                            source: anyhow::Error::new(stream_error),
                        })?;

                    file.write_all(&chunk)
                        .await
                        .map_err(|source| self.cache_failure(source))?;
                    (self.on_progress)(DownloadProgress::ChunkWritten {
                        byte_count: chunk.len() as u64,
                    });
                }
            }
        };

        file.flush()
            .await
            .map_err(|source| self.cache_failure(source))?;

        Ok(outcome)
    }
}

pub struct DownloadManager {
    client: Client,
}

impl DownloadManager {
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self { client })
    }

    pub async fn download<TProgressHandler>(
        &self,
        cancellation_token: &CancellationToken,
        url: &str,
        final_path: &Path,
        on_progress: &TProgressHandler,
    ) -> Result<DownloadOutcome, DownloadError>
    where
        TProgressHandler: Fn(DownloadProgress) + Sync,
    {
        let parsed_url = Url::parse(url).map_err(|source| DownloadError::InvalidUrl {
            url: url.to_owned(),
            source,
        })?;

        if !matches!(parsed_url.scheme(), "http" | "https") {
            return Err(DownloadError::UnsupportedUrlScheme {
                url: url.to_owned(),
                scheme: parsed_url.scheme().to_owned(),
            });
        }

        PartialFileDownload {
            cancellation_token,
            on_progress,
            partial: PartialFile::new(final_path.to_path_buf()),
            url,
        }
        .run(&self.client)
        .await
    }
}
