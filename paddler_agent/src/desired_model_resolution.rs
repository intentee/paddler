use std::path::PathBuf;

#[derive(Debug)]
pub enum DesiredModelResolution {
    Cancelled,
    NotConfigured,
    Resolved(PathBuf),
    LocalFileMissing(PathBuf),
}
