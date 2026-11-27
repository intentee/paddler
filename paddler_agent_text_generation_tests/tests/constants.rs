#![cfg(feature = "tests_that_use_llms")]

use paddler_file_fixture::image_fixture::ImageFixture;

pub const JPEG_AND_WEBP_IMAGE_FIXTURES: [ImageFixture; 2] =
    [ImageFixture::SarnowJpeg, ImageFixture::LlamasWebp];
pub const WEATHER_TOOL_CALL_JSON: &str =
    r#"{"name": "get_weather", "arguments": {"location": "Paris"}}"#;
