#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadProgress {
    ChunkWritten {
        byte_count: u64,
    },
    Finished,
    Started {
        already_downloaded_bytes: u64,
        total_bytes: Option<u64>,
    },
}
