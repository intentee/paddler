use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum DesiredStateVersion {
    #[default]
    Nothing,
    Version(u64),
}

impl DesiredStateVersion {
    #[must_use]
    pub const fn precedes(&self, version: u64) -> bool {
        match self {
            Self::Nothing => true,
            Self::Version(delivered_version) => *delivered_version < version,
        }
    }
}
