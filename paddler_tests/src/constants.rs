use std::num::NonZeroU32;
use std::time::Duration;

use crate::image_fixture::ImageFixture;

pub const ANSWERED_REQUEST_ID: &str = "model-metadata";
pub const DIMENSIONS_TOO_MANY_FOR_RMS_NORM: usize = 1 << 16;
pub const GRAMMAR_REFUSING_THE_GREEDY_SELECTION_PROMPT: &str = "Paddler balances";
pub const JPEG_AND_WEBP_IMAGE_FIXTURES: [ImageFixture; 2] = [
    ImageFixture {
        fixture_name: "sarnow.jpeg",
        mime_type: "image/jpeg",
    },
    ImageFixture {
        fixture_name: "llamas.webp",
        mime_type: "image/webp",
    },
];
pub const QUESTIONS_OUTLASTING_A_DISCONNECT: usize = 96;
pub const QWEN3_0_6B_UNUSED_PADDING_TOKEN: u32 = 151_669;
pub const SHORT_BUFFERED_REQUEST_TIMEOUT: Duration = Duration::from_millis(50);
pub const SMALL_DECISION_CONTEXT_SIZE: NonZeroU32 = NonZeroU32::new(256).unwrap();
pub const TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID: &str = "typesafe-request";
pub const UNCOMPILABLE_CHAT_TEMPLATE_CONTENT: &str = "{{invalid jinja template";
pub const UNCOMPILABLE_CHAT_TEMPLATE_PARSE_FAILURE: &str =
    "parser: Parser Error: Expected }} (Got jinja)\n...{{invalid jinja template...\n             ^";
pub const WEATHER_TOOL_CALL_JSON: &str =
    r#"{"name": "get_weather", "arguments": {"location": "Paris"}}"#;
pub const WEATHER_TOOL_QUESTION: &str =
    "What is the weather in Paris? Use the get_weather tool to find out.";
