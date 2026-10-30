use std::fs::read;

use anyhow::Context as _;
use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

use crate::fixture_path::fixture_path;

pub fn load_fixture_data_uri(fixture_name: &str, mime_type: &str) -> Result<String> {
    let fixture_path = fixture_path(fixture_name);
    let fixture_bytes = read(&fixture_path)
        .with_context(|| format!("failed to read test fixture {}", fixture_path.display()))?;

    let encoded = BASE64_STANDARD.encode(&fixture_bytes);

    Ok(format!("data:{mime_type};base64,{encoded}"))
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::io::ErrorKind;

    use super::load_fixture_data_uri;

    #[test]
    fn a_missing_fixture_is_reported_as_not_found() {
        let error = load_fixture_data_uri("missing_fixture.bin", "application/octet-stream")
            .expect_err("a missing fixture must not load");

        assert_eq!(
            error.downcast_ref::<io::Error>().map(io::Error::kind),
            Some(ErrorKind::NotFound)
        );
    }
}
