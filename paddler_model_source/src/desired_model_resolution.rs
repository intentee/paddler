use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq)]
pub enum DesiredModelResolution {
    Cancelled,
    Resolved(PathBuf),
    LocalFileMissing(PathBuf),
}
