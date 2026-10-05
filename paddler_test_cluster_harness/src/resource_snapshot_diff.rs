pub struct ResourceSnapshotDiff {
    pub open_file_descriptors_grew_by: usize,
}

impl ResourceSnapshotDiff {
    #[must_use]
    pub fn pretty_summary(&self) -> String {
        format!(
            "open file descriptors grew by {growth}",
            growth = self.open_file_descriptors_grew_by,
        )
    }
}
