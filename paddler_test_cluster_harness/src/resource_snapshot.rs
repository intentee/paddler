use std::fs::DirEntry;
use std::fs::read_dir;
use std::io;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::resource_snapshot_diff::ResourceSnapshotDiff;

#[cfg(target_os = "macos")]
const fn open_descriptors_directory_path() -> &'static str {
    "/dev/fd"
}

#[cfg(target_os = "linux")]
const fn open_descriptors_directory_path() -> &'static str {
    "/proc/self/fd"
}

pub struct ResourceSnapshot {
    pub open_file_descriptor_count: usize,
}

impl ResourceSnapshot {
    pub fn try_from_self() -> Result<Self, ClusterHarnessError> {
        let open_file_descriptors = read_dir(open_descriptors_directory_path())
            .and_then(Iterator::collect::<Result<Vec<DirEntry>, io::Error>>)
            .map_err(ClusterHarnessError::OpenFileDescriptorsUnreadable)?;

        Ok(Self {
            open_file_descriptor_count: open_file_descriptors.len(),
        })
    }

    #[must_use]
    pub const fn diff(&self, earlier: &Self) -> ResourceSnapshotDiff {
        ResourceSnapshotDiff {
            open_file_descriptors_grew_by: self
                .open_file_descriptor_count
                .saturating_sub(earlier.open_file_descriptor_count),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ResourceSnapshot;

    #[test]
    fn try_from_self_counts_the_processes_open_descriptors() {
        let snapshot = ResourceSnapshot::try_from_self()
            .expect("the current process must be able to list its open file descriptors");

        assert!(snapshot.open_file_descriptor_count > 0);
    }

    #[test]
    fn diff_reports_growth() {
        let later = ResourceSnapshot {
            open_file_descriptor_count: 10,
        };
        let earlier = ResourceSnapshot {
            open_file_descriptor_count: 3,
        };

        assert_eq!(later.diff(&earlier).open_file_descriptors_grew_by, 7);
    }

    #[test]
    fn diff_saturates_when_descriptors_shrink() {
        let later = ResourceSnapshot {
            open_file_descriptor_count: 3,
        };
        let earlier = ResourceSnapshot {
            open_file_descriptor_count: 10,
        };

        assert_eq!(later.diff(&earlier).open_file_descriptors_grew_by, 0);
    }
}
