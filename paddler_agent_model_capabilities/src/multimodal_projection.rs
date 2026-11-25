use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MultimodalProjection {
    File(PathBuf),
    NotConfigured,
}
