use anyhow::Result;

use paddler_test_cluster_harness::load_fixture_data_uri::load_fixture_data_uri;

pub struct ImageFixture {
    pub fixture_name: &'static str,
    pub mime_type: &'static str,
}

impl ImageFixture {
    /// # Errors
    ///
    /// Returns the error of [`load_fixture_data_uri`] when the image fixture cannot be read.
    pub fn data_uri(&self) -> Result<String> {
        load_fixture_data_uri(self.fixture_name, self.mime_type)
    }
}
