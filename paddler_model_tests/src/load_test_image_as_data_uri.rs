use std::fs;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

#[must_use]
#[expect(clippy::expect_used, reason = "test fixture must be loadable")]
pub fn load_test_image_as_data_uri() -> String {
    let image_bytes = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/llamas.jpg"
    ))
    .expect("Failed to read test fixture llamas.jpg");

    let encoded = BASE64_STANDARD.encode(&image_bytes);

    format!("data:image/jpeg;base64,{encoded}")
}
