use std::fs::read;

use anyhow::Context as _;
use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

pub fn load_fixture_data_uri(fixture_name: &str, mime_type: &str) -> Result<String> {
    let fixture_path = format!("{}/../fixtures/{fixture_name}", env!("CARGO_MANIFEST_DIR"));
    let fixture_bytes = read(&fixture_path)
        .with_context(|| format!("failed to read test fixture {fixture_path}"))?;

    let encoded = BASE64_STANDARD.encode(&fixture_bytes);

    Ok(format!("data:{mime_type};base64,{encoded}"))
}
