use crate::agent::grammar_sampler::GrammarSampler;
use crate::decoded_image::DecodedImage;

pub enum PreparedConversationHistoryRequest {
    TextPrompt {
        raw_prompt: String,
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
    },
    MultimodalPrompt {
        raw_prompt: String,
        images: Vec<DecodedImage>,
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
    },
}
