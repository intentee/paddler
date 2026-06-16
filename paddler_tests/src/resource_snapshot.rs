use anyhow::Context as _;
use anyhow::Result;
use tokio::fs::read_dir;

use crate::resource_snapshot_diff::ResourceSnapshotDiff;

pub struct ResourceSnapshot {
    pub open_file_descriptor_count: usize,
}

impl ResourceSnapshot {
    pub async fn try_from_self() -> Result<Self> {
        let directory_path = open_descriptors_directory_path();

        let mut entries = read_dir(directory_path).await.with_context(|| {
            format!(
                "failed to read open-descriptors directory {directory_path:?} for the current process"
            )
        })?;

        let mut open_file_descriptor_count: usize = 0;

        while entries
            .next_entry()
            .await
            .context("failed to enumerate an open-descriptor entry for the current process")?
            .is_some()
        {
            open_file_descriptor_count += 1;
        }

        Ok(Self {
            open_file_descriptor_count,
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

#[cfg(target_os = "macos")]
const fn open_descriptors_directory_path() -> &'static str {
    "/dev/fd"
}

#[cfg(target_os = "linux")]
const fn open_descriptors_directory_path() -> &'static str {
    "/proc/self/fd"
}

#[cfg(test)]
mod tests {
    use super::ResourceSnapshot;

    #[tokio::test]
    async fn try_from_self_counts_the_processes_open_descriptors() {
        let snapshot = ResourceSnapshot::try_from_self().await.unwrap();

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
