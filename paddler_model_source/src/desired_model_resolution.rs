use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq)]
pub enum DesiredModelResolution {
    Cancelled,
    NotConfigured,
    Resolved(PathBuf),
    LocalFileMissing(PathBuf),
}
