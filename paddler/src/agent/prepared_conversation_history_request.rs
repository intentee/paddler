use crate::request_params::continue_from_conversation_history_params::tool::Tool;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

use crate::agent::grammar_sampler::GrammarSampler;
use crate::decoded_image::DecodedImage;

pub enum PreparedConversationHistoryRequest {
    TextPrompt {
        raw_prompt: String,
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
        parse_tool_calls: bool,
        tools: Vec<Tool<ValidatedParametersSchema>>,
    },
    MultimodalPrompt {
        raw_prompt: String,
        images: Vec<DecodedImage>,
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
        parse_tool_calls: bool,
        tools: Vec<Tool<ValidatedParametersSchema>>,
    },
}
