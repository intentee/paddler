use std::fs::read_to_string;
use std::path::PathBuf;

use crate::tcp_buffer_limits_error::TcpBufferLimitsError;

const KERNEL_TCP_RECEIVE_BUFFER_LIMITS: &str = "/proc/sys/net/ipv4/tcp_rmem";
const KERNEL_TCP_SEND_BUFFER_LIMITS: &str = "/proc/sys/net/ipv4/tcp_wmem";

pub struct TcpBufferLimitsFile {
    pub path: PathBuf,
}

impl TcpBufferLimitsFile {
    #[must_use]
    pub fn of_kernel_receive_buffers() -> Self {
        Self {
            path: PathBuf::from(KERNEL_TCP_RECEIVE_BUFFER_LIMITS),
        }
    }

    #[must_use]
    pub fn of_kernel_send_buffers() -> Self {
        Self {
            path: PathBuf::from(KERNEL_TCP_SEND_BUFFER_LIMITS),
        }
    }

    pub fn largest_buffer(&self) -> Result<usize, TcpBufferLimitsError> {
        let limits = read_to_string(&self.path).map_err(|source| {
            TcpBufferLimitsError::LimitsUnreadable {
                path: self.path.clone(),
                source,
            }
        })?;
        let byte_counts = limits.split_whitespace().collect::<Vec<_>>();
        let [_smallest_limit, _default_limit, largest_limit] = byte_counts.as_slice() else {
            return Err(TcpBufferLimitsError::LimitsAreNotThreeByteCounts {
                limits: limits.clone(),
                path: self.path.clone(),
            });
        };

        largest_limit
            .parse()
            .map_err(|source| TcpBufferLimitsError::LargestLimitIsNotAByteCount {
                largest_limit: (*largest_limit).to_owned(),
                path: self.path.clone(),
                source,
            })
    }
}

#[cfg(test)]
mod tests {
    use std::fs::write;

    use tempfile::TempDir;

    use super::TcpBufferLimitsFile;
    use crate::tcp_buffer_limits_error::TcpBufferLimitsError;

    fn limits_file_containing(directory: &TempDir, limits: &str) -> TcpBufferLimitsFile {
        let path = directory.path().join("tcp_wmem");

        write(&path, limits).unwrap();

        TcpBufferLimitsFile { path }
    }

    #[test]
    fn the_largest_buffer_is_the_last_of_the_three_limits() {
        let directory = TempDir::new().unwrap();
        let limits_file = limits_file_containing(&directory, "4096\t16384\t4194304\n");

        assert_eq!(limits_file.largest_buffer().unwrap(), 4_194_304);
    }

    #[test]
    fn a_missing_limits_file_is_unreadable() {
        let directory = TempDir::new().unwrap();
        let limits_file = TcpBufferLimitsFile {
            path: directory.path().join("missing"),
        };

        assert!(matches!(
            limits_file.largest_buffer(),
            Err(TcpBufferLimitsError::LimitsUnreadable { path, .. }) if path == limits_file.path
        ));
    }

    #[test]
    fn limits_other_than_three_values_are_rejected() {
        let directory = TempDir::new().unwrap();
        let limits_file = limits_file_containing(&directory, "4096\t16384\n");

        assert!(matches!(
            limits_file.largest_buffer(),
            Err(TcpBufferLimitsError::LimitsAreNotThreeByteCounts { limits, .. }) if limits == "4096\t16384\n"
        ));
    }

    #[test]
    fn a_largest_limit_that_is_not_a_byte_count_is_rejected() {
        let directory = TempDir::new().unwrap();
        let limits_file = limits_file_containing(&directory, "4096\t16384\tunbounded\n");

        assert!(matches!(
            limits_file.largest_buffer(),
            Err(TcpBufferLimitsError::LargestLimitIsNotAByteCount { largest_limit, .. }) if largest_limit == "unbounded"
        ));
    }
}
