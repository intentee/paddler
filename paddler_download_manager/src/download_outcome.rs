#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadOutcome {
    Completed,
    Cancelled,
}
