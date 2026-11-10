pub enum DeliveredDesiredStateVersion {
    Nothing,
    Version(u64),
}

impl DeliveredDesiredStateVersion {
    #[must_use]
    pub const fn precedes(&self, version: u64) -> bool {
        match self {
            Self::Nothing => true,
            Self::Version(delivered_version) => *delivered_version < version,
        }
    }
}
