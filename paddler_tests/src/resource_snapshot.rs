use std::fs;

use anyhow::Context as _;
use anyhow::Result;

use crate::resource_snapshot_diff::ResourceSnapshotDiff;

pub struct ResourceSnapshot {
    pub open_file_descriptor_count: usize,
}

impl ResourceSnapshot {
    pub fn try_from_self() -> Result<Self> {
        let directory_path = open_descriptors_directory_path();

        let entries = fs::read_dir(directory_path).with_context(|| {
            format!(
                "failed to read open-descriptors directory {directory_path:?} for the current process"
            )
        })?;

        let mut open_file_descriptor_count: usize = 0;

        for entry_result in entries {
            entry_result
                .context("failed to enumerate an open-descriptor entry for the current process")?;

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

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
compile_error!("ResourceSnapshot is only implemented for macOS and Linux");
