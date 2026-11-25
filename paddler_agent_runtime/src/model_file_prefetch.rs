use std::fs::File;
use std::io;
use std::io::Read as _;
use std::io::Seek as _;
use std::io::SeekFrom;
use std::io::copy;
use std::io::sink;
use std::iter::from_fn;
use std::iter::successors;
use std::num::NonZeroUsize;
use std::path::Path;
use std::thread::scope;

use parking_lot::Mutex;

use crate::agent_runtime_error::AgentRuntimeError;

const PREFETCH_CHUNK_BYTES: u64 = 8 * 1024 * 1024;

fn prefetch_chunk(model_file: &File, chunk_offset: u64) -> io::Result<u64> {
    let mut chunk_reader = model_file;

    chunk_reader
        .seek(SeekFrom::Start(chunk_offset))
        .and_then(|_chunk_position| copy(&mut chunk_reader.take(PREFETCH_CHUNK_BYTES), &mut sink()))
}

fn prefetch_chunks<TChunkOffsets>(
    model_path: &Path,
    chunk_offsets: &Mutex<TChunkOffsets>,
) -> io::Result<()>
where
    TChunkOffsets: Iterator<Item = u64>,
{
    File::open(model_path).and_then(|model_file| {
        from_fn(|| chunk_offsets.lock().next())
            .map(|chunk_offset| prefetch_chunk(&model_file, chunk_offset))
            .take_while(|prefetched_bytes| !matches!(prefetched_bytes, Ok(0)))
            .try_for_each(|prefetched_bytes| prefetched_bytes.map(drop))
    })
}

pub struct ModelFilePrefetch<'model_file> {
    pub model_path: &'model_file Path,
    pub stream_count: NonZeroUsize,
}

impl ModelFilePrefetch<'_> {
    /// # Errors
    ///
    /// Returns the error of `load` when `load` fails, and [`AgentRuntimeError::ModelFilePrefetchFailed`] when `load` succeeds but the model file cannot be read.
    pub fn alongside<TLoaded, TLoadError>(
        self,
        load: impl FnOnce() -> Result<TLoaded, TLoadError>,
    ) -> Result<TLoaded, TLoadError>
    where
        TLoadError: From<AgentRuntimeError>,
    {
        let Self {
            model_path,
            stream_count,
        } = self;
        let chunk_offsets = Mutex::new(successors(Some(0_u64), |chunk_offset| {
            chunk_offset.checked_add(PREFETCH_CHUNK_BYTES)
        }));
        let prefetch_errors = Mutex::new(Vec::new());
        let loaded = scope(|prefetch_scope| {
            for _prefetch_stream in 0..stream_count.get() {
                prefetch_scope.spawn(|| {
                    if let Err(prefetch_error) = prefetch_chunks(model_path, &chunk_offsets) {
                        prefetch_errors.lock().push(prefetch_error);
                    }
                });
            }

            load()
        });

        loaded.and_then(|loaded_model| {
            prefetch_errors
                .into_inner()
                .into_iter()
                .next()
                .map_or(Ok(loaded_model), |source| {
                    Err(AgentRuntimeError::ModelFilePrefetchFailed {
                        model_path: model_path.to_path_buf(),
                        source,
                    }
                    .into())
                })
        })
    }
}
