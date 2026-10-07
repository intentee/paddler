use std::path::PathBuf;

pub fn fixture_path(fixture_name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../fixtures/{fixture_name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}
