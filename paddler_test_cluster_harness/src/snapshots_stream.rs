#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotsStream {
    Agents,
    BufferedRequests,
}
