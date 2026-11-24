use std::path::PathBuf;

#[must_use]
pub fn fixture_path(fixture_file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .with_file_name("fixtures")
        .join(fixture_file_name)
}
