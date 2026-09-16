use serde_json::Value;

use crate::decoded_image::DecodedImage;
use crate::grammar_sampler::GrammarSampler;

pub enum PreparedConversationHistoryRequest {
    TextPrompt {
        raw_prompt: String,
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
        parse_tool_calls: bool,
        tools: Vec<Value>,
    },
    MultimodalPrompt {
        raw_prompt: String,
        images: Vec<DecodedImage>,
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
        parse_tool_calls: bool,
        tools: Vec<Value>,
    },
}
