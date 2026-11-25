use paddler_file_fixture::constants::LLAMAS_WEBP;
use paddler_file_fixture::constants::SARNOW_JPEG;
use paddler_file_fixture::image_fixture::ImageFixture;

pub const CHAT_TEMPLATE_GGUF_KEY: &str = "tokenizer.chat_template";
pub const GRAMMAR_REFUSING_THE_GREEDY_SELECTION_PROMPT: &str = "Paddler balances";
pub const JPEG_AND_WEBP_IMAGE_FIXTURES: [ImageFixture; 2] = [SARNOW_JPEG, LLAMAS_WEBP];
pub const QWEN3_0_6B_UNUSED_PADDING_TOKEN: u32 = 151_669;
pub const WEATHER_TOOL_CALL_JSON: &str =
    r#"{"name": "get_weather", "arguments": {"location": "Paris"}}"#;
