use std::io;
use std::num::ParseIntError;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TcpBufferLimitsError {
    #[error("The TCP buffer limits in '{path_display}' are not exactly a minimum, a default, and a maximum: '{limits}'", path_display = path.display())]
    LimitsAreNotThreeByteCounts { limits: String, path: PathBuf },
    #[error("Unable to read the TCP buffer limits from '{path_display}'", path_display = path.display())]
    LimitsUnreadable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("The largest TCP buffer limit '{largest_limit}' in '{path_display}' is not a byte count", path_display = path.display())]
    LargestLimitIsNotAByteCount {
        largest_limit: String,
        path: PathBuf,
        #[source]
        source: ParseIntError,
    },
}
