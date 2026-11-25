use std::fs::read;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

use crate::file_fixture_error::FileFixtureError;
use crate::fixture_path::fixture_path;

/// # Errors
///
/// Returns [`FileFixtureError::FixtureUnreadable`] when the fixture cannot be read.
pub fn fixture_base64(fixture_name: &str) -> Result<String, FileFixtureError> {
    let path = fixture_path(fixture_name);
    let fixture_bytes =
        read(&path).map_err(|source| FileFixtureError::FixtureUnreadable { path, source })?;

    Ok(BASE64_STANDARD.encode(fixture_bytes))
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use super::fixture_base64;
    use crate::file_fixture_error::FileFixtureError;

    #[test]
    fn a_missing_fixture_is_reported_as_not_found() {
        let error =
            fixture_base64("missing_fixture.bin").expect_err("a missing fixture must not load");

        assert!(matches!(
            error,
            FileFixtureError::FixtureUnreadable { source, .. } if source.kind() == ErrorKind::NotFound
        ));
    }
}
