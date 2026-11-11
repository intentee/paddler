use std::io;
use std::path::Path;
use std::time::Duration;

use bytes::Bytes;
use futures_util::Stream;
use futures_util::StreamExt as _;
use headers::ContentRange;
use headers::HeaderMapExt as _;
use reqwest::Client;
use reqwest::Error as ReqwestError;
use reqwest::Response;
use reqwest::header::RANGE;
use tokio::fs::File;
use tokio::io::AsyncWriteExt as _;
use tokio_util::sync::CancellationToken;

use crate::constants::READ_TIMEOUT;
use crate::download_error::DownloadError;
use crate::download_outcome::DownloadOutcome;
use crate::download_progress::DownloadProgress;
use crate::download_url::DownloadUrl;
use crate::partial_file::PartialFile;
use crate::response_classification::ResponseClassification;

struct PartialFileDownload<'download, TProgressHandler> {
    cancellation_token: &'download CancellationToken,
    on_progress: &'download TProgressHandler,
    partial: PartialFile,
    url: &'download DownloadUrl,
}

impl<TProgressHandler> PartialFileDownload<'_, TProgressHandler>
where
    TProgressHandler: Fn(DownloadProgress) + Sync,
{
    fn cache_failure(&self, source: io::Error) -> DownloadError {
        DownloadError::cache_failure(self.partial.partial_path().to_path_buf(), source)
    }

    fn declared_total_bytes(
        &self,
        already_downloaded_bytes: u64,
        response: &Response,
    ) -> Result<Option<u64>, DownloadError> {
        response
            .content_length()
            .map(|content_length| {
                already_downloaded_bytes
                    .checked_add(content_length)
                    .ok_or_else(|| DownloadError::DeclaredContentLengthOverflowsTotalSize {
                        url: self.url.as_str().to_owned(),
                        already_downloaded_bytes,
                        content_length,
                    })
            })
            .transpose()
    }

    fn downloaded_bytes_after_chunk(
        &self,
        downloaded_bytes: u64,
        chunk: &Bytes,
    ) -> Result<u64, DownloadError> {
        let chunk_byte_count = chunk.len() as u64;

        downloaded_bytes
            .checked_add(chunk_byte_count)
            .ok_or_else(
                || DownloadError::DownloadedBytesOverflowTheLargestFileSize {
                    url: self.url.as_str().to_owned(),
                    downloaded_bytes,
                    chunk_byte_count,
                },
            )
    }

    async fn remove_stale_partial(&self) -> Result<DownloadOutcome, DownloadError> {
        self.partial
            .remove()
            .await
            .map_err(|source| self.cache_failure(source))?;

        Err(DownloadError::PartialFileStale {
            url: self.url.as_str().to_owned(),
            partial_path: self.partial.partial_path().to_path_buf(),
        })
    }

    async fn run(self, client: &Client) -> Result<DownloadOutcome, DownloadError> {
        let offset = self.partial.current_size().await?;
        let sent_range_header = offset > 0;
        let mut request = client.get(self.url.as_str());

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
                send_result.map_err(|source| DownloadError::DownloadServerIsUnreachable {
                    url: self.url.as_str().to_owned(),
                    source,
                })?
            }
        };

        match ResponseClassification::from_status(response.status(), sent_range_header) {
            ResponseClassification::NotFound => Err(DownloadError::NotFound {
                url: self.url.as_str().to_owned(),
            }),
            ResponseClassification::PermissionDenied(status) => {
                Err(DownloadError::PermissionDenied {
                    url: self.url.as_str().to_owned(),
                    status,
                })
            }
            ResponseClassification::PartialFileStale => self.remove_stale_partial().await,
            ResponseClassification::ServerError(status) => {
                Err(DownloadError::DownloadServerErrored {
                    url: self.url.as_str().to_owned(),
                    status,
                })
            }
            ResponseClassification::ClientError(status) => {
                Err(DownloadError::DownloadServerRejectedRequest {
                    url: self.url.as_str().to_owned(),
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
        let total_bytes = self.declared_total_bytes(offset, &response)?;

        (self.on_progress)(DownloadProgress::Started {
            already_downloaded_bytes: offset,
            total_bytes,
        });

        let mut file = self
            .partial
            .open_for_append()
            .await
            .map_err(|source| self.cache_failure(source))?;
        let stream_outcome = self
            .write_body_stream(response.bytes_stream(), &mut file, offset)
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
        already_downloaded_bytes: u64,
    ) -> Result<DownloadOutcome, DownloadError>
    where
        TStream: Stream<Item = Result<Bytes, ReqwestError>> + Unpin,
    {
        let mut downloaded_bytes = already_downloaded_bytes;
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
                        next_chunk.map_err(|source| DownloadError::DownloadInterrupted {
                            url: self.url.as_str().to_owned(),
                            source,
                        })?;

                    downloaded_bytes =
                        self.downloaded_bytes_after_chunk(downloaded_bytes, &chunk)?;
                    file.write_all(&chunk)
                        .await
                        .map_err(|source| self.cache_failure(source))?;
                    (self.on_progress)(DownloadProgress::ChunkWritten { downloaded_bytes });
                }
            }
        };

        file.flush()
            .await
            .map_err(|source| self.cache_failure(source))?;

        Ok(outcome)
    }
}

#[derive(Clone)]
pub struct DownloadManager {
    client: Client,
}

impl DownloadManager {
    /// # Errors
    ///
    /// Returns a [`ReqwestError`] when the HTTP client cannot be built, for example because the TLS backend fails to initialize.
    pub fn new() -> Result<Self, ReqwestError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(READ_TIMEOUT)
            .build()?;

        Ok(Self { client })
    }

    /// # Errors
    ///
    /// Returns [`DownloadError::FinalPathHasNoFileName`] when the final path does not name a file, [`DownloadError::PartialPathIsADirectory`] when a directory occupies the partial file path, [`DownloadError::DownloadServerIsUnreachable`] when the request cannot be sent, [`DownloadError::NotFound`], [`DownloadError::PermissionDenied`], [`DownloadError::DownloadServerRejectedRequest`] or [`DownloadError::DownloadServerErrored`] when the server answers with a 404, a 401 or 403, another client error, or a server error, [`DownloadError::PartialFileStale`] when the server cannot satisfy the range of the partial file or answers with a different range, [`DownloadError::DeclaredContentLengthOverflowsTotalSize`] when the declared content length added to the already downloaded bytes does not fit in a `u64`, [`DownloadError::DownloadInterrupted`] when the body stream fails, and [`DownloadError::CachePermissionDenied`], [`DownloadError::CacheDiskFull`] or [`DownloadError::Io`] when the partial or final file cannot be read, written, removed or renamed.
    pub async fn download<TProgressHandler>(
        &self,
        cancellation_token: &CancellationToken,
        url: &DownloadUrl,
        final_path: &Path,
        on_progress: &TProgressHandler,
    ) -> Result<DownloadOutcome, DownloadError>
    where
        TProgressHandler: Fn(DownloadProgress) + Sync,
    {
        PartialFileDownload {
            cancellation_token,
            on_progress,
            partial: PartialFile::new(final_path.to_path_buf())?,
            url,
        }
        .run(&self.client)
        .await
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures_util::stream::iter;
    use reqwest::Error as ReqwestError;
    use tempfile::tempdir;
    use tokio::fs::File;
    use tokio::fs::read;
    use tokio_util::sync::CancellationToken;

    use super::PartialFileDownload;
    use crate::download_error::DownloadError;
    use crate::download_progress::DownloadProgress;
    use crate::download_url::DownloadUrl;
    use crate::partial_file::PartialFile;

    const DOWNLOADED_URL: &str = "https://example.com/model.gguf";

    #[tokio::test]
    async fn a_chunk_overflowing_the_largest_file_size_fails_before_it_is_written() {
        let cache_directory = tempdir().unwrap();
        let partial = PartialFile::new(cache_directory.path().join("model.gguf")).unwrap();
        let partial_path = partial.partial_path().to_path_buf();
        let mut partial_file = File::create(&partial_path).await.unwrap();
        let url = DownloadUrl::parse(DOWNLOADED_URL).unwrap();

        let write_result = PartialFileDownload {
            cancellation_token: &CancellationToken::new(),
            on_progress: &drop::<DownloadProgress>,
            partial,
            url: &url,
        }
        .write_body_stream(
            iter([Ok::<Bytes, ReqwestError>(Bytes::from_static(b"x"))]),
            &mut partial_file,
            u64::MAX,
        )
        .await;

        assert!(matches!(
            write_result,
            Err(DownloadError::DownloadedBytesOverflowTheLargestFileSize {
                url,
                downloaded_bytes: u64::MAX,
                chunk_byte_count: 1,
            }) if url == DOWNLOADED_URL
        ));
        assert_eq!(read(&partial_path).await.unwrap(), Vec::<u8>::new());
    }
}
