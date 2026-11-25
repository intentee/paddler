use std::fs::read;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

use crate::file_fixture_error::FileFixtureError;
use crate::fixture_path::fixture_path;

pub struct ImageFixture {
    pub fixture_name: &'static str,
    pub mime_type: &'static str,
}

impl ImageFixture {
    /// # Errors
    ///
    /// Returns [`FileFixtureError::FixtureUnreadable`] when the image fixture cannot be read.
    pub fn data_uri(&self) -> Result<String, FileFixtureError> {
        let path = fixture_path(self.fixture_name);
        let fixture_bytes =
            read(&path).map_err(|source| FileFixtureError::FixtureUnreadable { path, source })?;

        Ok(format!(
            "data:{};base64,{}",
            self.mime_type,
            BASE64_STANDARD.encode(fixture_bytes)
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use super::ImageFixture;
    use crate::file_fixture_error::FileFixtureError;

    #[test]
    fn a_missing_fixture_is_reported_as_not_found() {
        let error = ImageFixture {
            fixture_name: "missing_fixture.bin",
            mime_type: "application/octet-stream",
        }
        .data_uri()
        .expect_err("a missing fixture must not load");

        assert!(matches!(
            error,
            FileFixtureError::FixtureUnreadable { source, .. } if source.kind() == ErrorKind::NotFound
        ));
    }
}
