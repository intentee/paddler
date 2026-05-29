use std::fs;

use anyhow::Context as _;
use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

pub fn load_test_image_data_uri() -> Result<String> {
    let image_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/llamas.jpg");
    let image_bytes = fs::read(image_path)
        .with_context(|| format!("failed to read test fixture {image_path}"))?;

    let encoded = BASE64_STANDARD.encode(&image_bytes);

    Ok(format!("data:image/jpeg;base64,{encoded}"))
}
