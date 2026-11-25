use crate::file_fixture_error::FileFixtureError;
use crate::fixture_base64::fixture_base64;

pub struct ImageFixture {
    pub fixture_name: &'static str,
    pub mime_type: &'static str,
}

impl ImageFixture {
    /// # Errors
    ///
    /// Returns [`FileFixtureError::FixtureUnreadable`] when the image fixture cannot be read.
    pub fn data_uri(&self) -> Result<String, FileFixtureError> {
        Ok(format!(
            "data:{};base64,{}",
            self.mime_type,
            fixture_base64(self.fixture_name)?
        ))
    }
}
