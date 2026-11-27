use std::path::PathBuf;

use crate::fixture_path::fixture_path;

#[must_use]
pub fn ca_bundle_without_certificates_path() -> PathBuf {
    fixture_path("ca_bundle_without_certificates.pem")
}
