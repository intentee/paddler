use std::path::PathBuf;

#[derive(Debug)]
pub enum DesiredModelResolution {
    NotConfigured,
    Resolved(PathBuf),
    LocalFileMissing(PathBuf),
}
