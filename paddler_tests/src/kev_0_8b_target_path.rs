use std::path::PathBuf;

#[must_use]
pub fn kev_0_8b_target_path(artifact_file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .with_file_name("target")
        .join("kev_0_8b")
        .join(artifact_file_name)
}
