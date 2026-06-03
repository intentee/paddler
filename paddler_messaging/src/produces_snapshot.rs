use anyhow::Result;
use serde::Serialize;

pub trait ProducesSnapshot {
    type Snapshot: Serialize;

    fn make_snapshot(&self) -> Result<Self::Snapshot>;
}
