use std::path::PathBuf;

pub enum DesiredModelResolution {
    NotConfigured,
    Resolved(PathBuf),
    LocalFileMissing(PathBuf),
}
