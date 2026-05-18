use std::sync::Arc;

use bytes::Bytes;
use futures_util::Stream;
use futures_util::StreamExt as _;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt as _;

use crate::progress_sink::ProgressSink;
use crate::stream_to_partial_file_error::StreamToPartialFileError;

pub async fn stream_to_partial_file<TStream, TWriter>(
    mut body_stream: TStream,
    writer: &mut TWriter,
    progress_sink: &Arc<dyn ProgressSink>,
) -> Result<(), StreamToPartialFileError>
where
    TStream: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
    TWriter: AsyncWrite + Unpin,
{
    while let Some(next_chunk) = body_stream.next().await {
        let bytes = next_chunk.map_err(StreamToPartialFileError::Stream)?;

        writer
            .write_all(&bytes)
            .await
            .map_err(StreamToPartialFileError::Write)?;

        progress_sink.on_chunk(bytes.len() as u64);
    }

    writer
        .flush()
        .await
        .map_err(StreamToPartialFileError::Write)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use anyhow::Result;
    use bytes::Bytes;
    use futures_util::stream;
    use tempfile::TempDir;
    use tokio::fs::OpenOptions;

    use crate::progress_sink::ProgressSink;
    use crate::stream_to_partial_file::stream_to_partial_file;

    struct CountingSink {
        chunks: AtomicU64,
        bytes: AtomicU64,
    }

    impl CountingSink {
        fn new() -> Self {
            Self {
                bytes: AtomicU64::new(0),
                chunks: AtomicU64::new(0),
            }
        }
    }

    impl ProgressSink for CountingSink {
        fn on_started(&self, _total_bytes: u64, _already_downloaded: u64) {}
        fn on_chunk(&self, additional_bytes: u64) {
            self.bytes.fetch_add(additional_bytes, Ordering::Relaxed);
            self.chunks.fetch_add(1, Ordering::Relaxed);
        }
        fn on_finished(&self) {}
    }

    #[test]
    fn counting_sink_lifecycle_methods_are_inert() {
        let sink = CountingSink::new();

        sink.on_started(1024, 0);
        sink.on_finished();

        assert_eq!(sink.chunks.load(Ordering::Relaxed), 0);
        assert_eq!(sink.bytes.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn writes_every_chunk_in_order() -> Result<()> {
        let directory = TempDir::new()?;
        let path = directory.path().join("dest.bin");
        let chunks: Vec<std::result::Result<Bytes, reqwest::Error>> = vec![
            Ok(Bytes::from_static(b"first")),
            Ok(Bytes::from_static(b"second")),
        ];
        let body_stream = stream::iter(chunks);
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await?;
        let sink: Arc<dyn ProgressSink> = Arc::new(CountingSink::new());

        stream_to_partial_file(body_stream, &mut file, &sink).await?;

        let bytes = tokio::fs::read(&path).await?;
        assert_eq!(bytes, b"firstsecond");

        Ok(())
    }

    #[tokio::test]
    async fn calls_progress_sink_once_per_chunk() -> Result<()> {
        let directory = TempDir::new()?;
        let path = directory.path().join("dest.bin");
        let chunks: Vec<std::result::Result<Bytes, reqwest::Error>> = vec![
            Ok(Bytes::from_static(b"aaa")),
            Ok(Bytes::from_static(b"bb")),
            Ok(Bytes::from_static(b"c")),
        ];
        let body_stream = stream::iter(chunks);
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await?;
        let counting = Arc::new(CountingSink::new());
        let sink: Arc<dyn ProgressSink> = counting.clone();

        stream_to_partial_file(body_stream, &mut file, &sink).await?;

        assert_eq!(counting.chunks.load(Ordering::Relaxed), 3);
        assert_eq!(counting.bytes.load(Ordering::Relaxed), 6);

        Ok(())
    }

    #[tokio::test]
    async fn write_to_closed_duplex_returns_error() {
        let (reader_half, mut writer_half) = tokio::io::duplex(0);
        drop(reader_half);

        let chunks: Vec<std::result::Result<Bytes, reqwest::Error>> =
            vec![Ok(Bytes::from_static(b"data"))];
        let body_stream = stream::iter(chunks);
        let sink: Arc<dyn ProgressSink> = Arc::new(CountingSink::new());

        let result = stream_to_partial_file(body_stream, &mut writer_half, &sink).await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn flush_to_read_only_file_returns_error() -> Result<()> {
        let directory = TempDir::new()?;
        let path = directory.path().join("read_only.bin");
        tokio::fs::write(&path, b"existing").await?;
        let chunks: Vec<std::result::Result<Bytes, reqwest::Error>> =
            vec![Ok(Bytes::from_static(b"more bytes"))];
        let body_stream = stream::iter(chunks);
        let mut read_only_file = OpenOptions::new().read(true).open(&path).await?;
        let sink: Arc<dyn ProgressSink> = Arc::new(CountingSink::new());

        let result = stream_to_partial_file(body_stream, &mut read_only_file, &sink).await;

        assert!(result.is_err());

        Ok(())
    }
}
