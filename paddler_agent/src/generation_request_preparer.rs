use std::sync::Arc;

use llama_cpp_bindings::ChatTools;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::mtmd::MtmdBitmap;
use paddler_image_decoder::decoded_image::DecodedImage;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

use crate::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::converts_to_mtmd_bitmap::ConvertsToMtmdBitmap;
use crate::generation_request_rejection::GenerationRequestRejection;
use crate::image_input::ImageInput;
use crate::prepared_generation_request::PreparedGenerationRequest;
use crate::prepared_prompt::PreparedPrompt;
use crate::prompt_tokenizer::PromptTokenizer;
use crate::resolve_grammar::resolve_grammar;
use crate::token_generation::TokenGeneration;
use crate::token_generation_support::TokenGenerationSupport;
use crate::tool_call_pipeline::ToolCallPipeline;

pub struct GenerationRequestPreparer {
    pub image_input: ImageInput,
    pub image_resize_to_fit: u32,
    pub model: Arc<LlamaModel>,
    pub prompt_tokenizer: PromptTokenizer,
    pub token_generation: TokenGeneration,
}

impl GenerationRequestPreparer {
    pub fn prepare_raw_prompt(
        &self,
        ContinueFromRawPromptRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params:
                ContinueFromRawPromptParams {
                    grammar,
                    max_tokens,
                    raw_prompt,
                },
            slot_guard,
        }: ContinueFromRawPromptRequest,
    ) -> Result<PreparedGenerationRequest, GenerationRequestRejection> {
        let grammar_sampler = resolve_grammar(grammar.as_ref(), false)?;
        let TokenGenerationSupport {
            streaming_markers, ..
        } = self.token_generation.require_enabled()?;

        Ok(PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            grammar_sampler,
            max_tokens,
            prompt: PreparedPrompt::TextTokens(self.prompt_tokenizer.tokenize(&raw_prompt)?),
            slot_guard,
            streaming_markers: streaming_markers.clone(),
            tool_call_pipeline: None,
        })
    }

    pub fn prepare_conversation_history(
        &self,
        ContinueFromConversationHistoryRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params,
            slot_guard,
        }: ContinueFromConversationHistoryRequest,
    ) -> Result<PreparedGenerationRequest, GenerationRequestRejection> {
        let grammar_sampler = resolve_grammar(params.grammar.as_ref(), params.enable_thinking)?;
        let TokenGenerationSupport {
            chat_prompt_renderer,
            streaming_markers,
        } = self.token_generation.require_enabled()?;

        let bitmaps = params
            .conversation_history
            .extract_image_urls()
            .into_iter()
            .map(|image_url| self.decode_image_bitmap(image_url))
            .collect::<Result<Vec<MtmdBitmap>, GenerationRequestRejection>>()?;

        let raw_prompt = chat_prompt_renderer.render(&params)?;

        let ContinueFromConversationHistoryParams {
            max_tokens,
            parse_tool_calls,
            tools,
            ..
        } = params;

        let tool_call_pipeline = if parse_tool_calls && !tools.is_empty() {
            Some(self.build_tool_call_pipeline(&tools)?)
        } else {
            None
        };

        let prompt = if bitmaps.is_empty() {
            PreparedPrompt::TextTokens(self.prompt_tokenizer.tokenize(&raw_prompt)?)
        } else {
            PreparedPrompt::Multimodal(
                self.image_input
                    .prepare_multimodal_prompt(bitmaps, raw_prompt)?,
            )
        };

        Ok(PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            grammar_sampler,
            max_tokens,
            prompt,
            slot_guard,
            streaming_markers: streaming_markers.clone(),
            tool_call_pipeline,
        })
    }

    fn decode_image_bitmap(
        &self,
        image_url: &ImageUrl,
    ) -> Result<MtmdBitmap, GenerationRequestRejection> {
        DecodedImage::from_data_uri(image_url, self.image_resize_to_fit)
            .map_err(GenerationRequestRejection::ImageDecodingFailed)?
            .to_mtmd_bitmap()
            .map_err(GenerationRequestRejection::ImageBitmapCreationFailed)
    }

    fn build_tool_call_pipeline(
        &self,
        tools: &[Tool<ValidatedParametersSchema>],
    ) -> Result<ToolCallPipeline, GenerationRequestRejection> {
        let tools_json = serde_json::to_string(tools)
            .map_err(GenerationRequestRejection::ToolsSerializationFailed)?;

        Ok(ToolCallPipeline::new(
            self.model.clone(),
            ChatTools::from_json(tools_json)
                .map_err(GenerationRequestRejection::ChatToolsInvalid)?,
            ToolCallValidator::from_tools(tools)
                .map_err(GenerationRequestRejection::ToolSchemaInvalid)?,
        ))
    }
}
