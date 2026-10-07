use std::num::NonZeroU32;

use llama_cpp_bindings::BareJsonToolCalls;
use llama_cpp_bindings::ChatMessageParser;
use llama_cpp_bindings::mtmd::MtmdBitmap;
use rand::Rng as _;
use rand::rng;
use serde_json::to_string;

use paddler_image_decoder::decoded_image::DecodedImage;
use paddler_messaging::chat_template_conversation::ChatTemplateConversation;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand;

use crate::chat_prompt_render_request::ChatPromptRenderRequest;
use crate::converts_to_mtmd_bitmap::ConvertsToMtmdBitmap;
use crate::image_input::ImageInput;
use crate::prepared_generation_request::PreparedGenerationRequest;
use crate::prepared_prompt::PreparedPrompt;
use crate::prompt_modality::PromptModality;
use crate::prompt_tokenizer::PromptTokenizer;
use crate::require_grammar_compatible_with_thinking::require_grammar_compatible_with_thinking;
use crate::resolve_grammar::resolve_grammar;
use crate::sampler_chain_factory::SamplerChainFactory;
use crate::text_generation_error::TextGenerationError;
use crate::text_generation_request::TextGenerationRequest;
use crate::token_classification::TokenClassification;
use crate::token_generation_support::TokenGenerationSupport;
use crate::token_sampling::TokenSampling;
use crate::tool_call_handling::ToolCallHandling;
use crate::tool_call_pipeline::ToolCallPipeline;

pub struct GenerationRequestPreparer {
    pub image_input: ImageInput,
    pub image_resize_to_fit: NonZeroU32,
    pub loaded_llama_model: LoadedLlamaModel,
    pub prompt_tokenizer: PromptTokenizer,
    pub sampler_chain_factory: SamplerChainFactory,
    pub token_generation_support: TokenGenerationSupport,
}

impl GenerationRequestPreparer {
    pub fn prepare_raw_prompt(
        &self,
        AgentRequest {
            params:
                ContinueFromRawPromptParams {
                    grammar,
                    max_tokens,
                    raw_prompt,
                },
            response_tx: generated_tokens_tx,
            slot_guard,
            stop_rx: generate_tokens_stop_rx,
        }: AgentRequest<ContinueFromRawPromptParams, GeneratedTokenResult>,
    ) -> Result<PreparedGenerationRequest, TextGenerationError> {
        let TokenGenerationSupport {
            streaming_markers, ..
        } = &self.token_generation_support;
        let prompt = PreparedPrompt::TextTokens(self.prompt_tokenizer.tokenize(&raw_prompt)?);
        let token_sampling = self.build_token_sampling(grammar)?;

        Ok(PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            max_tokens,
            prompt,
            slot_guard,
            token_classification: TokenClassification {
                bare_json_tool_calls: BareJsonToolCalls::Ignore,
                streaming_markers: streaming_markers.clone(),
            },
            token_sampling,
            tool_call_handling: ToolCallHandling::Streamed,
        })
    }

    pub fn prepare_conversation_history(
        &self,
        AgentRequest {
            response_tx: generated_tokens_tx,
            stop_rx: generate_tokens_stop_rx,
            params:
                ContinueFromConversationHistoryParams {
                    add_generation_prompt,
                    conversation_history,
                    enable_thinking,
                    grammar,
                    max_tokens,
                    parse_tool_calls,
                    tools,
                },
            slot_guard,
        }: AgentRequest<
            ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
            GeneratedTokenResult,
        >,
    ) -> Result<PreparedGenerationRequest, TextGenerationError> {
        let TokenGenerationSupport {
            chat_prompt_renderer,
            streaming_markers,
        } = &self.token_generation_support;

        require_grammar_compatible_with_thinking(grammar.as_ref(), enable_thinking)?;

        let ChatTemplateConversation {
            image_urls,
            messages,
        } = conversation_history
            .into_chat_template_conversation(&chat_prompt_renderer.media_marker);
        let prompt_modality = self.image_input.prompt_modality_for(&image_urls)?;

        let raw_prompt = chat_prompt_renderer.render(ChatPromptRenderRequest {
            add_generation_prompt,
            enable_thinking,
            messages: &messages,
            tools: &tools,
        })?;

        let prompt = match prompt_modality {
            PromptModality::Multimodal(multimodal_prompt_support) => {
                let add_special_tokens = !self
                    .prompt_tokenizer
                    .renders_its_own_bos_token(&raw_prompt)?;

                PreparedPrompt::Multimodal(
                    multimodal_prompt_support.prepare_prompt(
                        &image_urls
                            .iter()
                            .map(|image_url| self.decode_image_bitmap(image_url))
                            .collect::<Result<Vec<MtmdBitmap>, TextGenerationError>>()?,
                        raw_prompt,
                        add_special_tokens,
                    )?,
                )
            }
            PromptModality::TextOnly => {
                PreparedPrompt::TextTokens(self.prompt_tokenizer.tokenize(&raw_prompt)?)
            }
        };

        let token_sampling = self.build_token_sampling(grammar)?;
        let tool_call_handling = if parse_tool_calls {
            ToolCallHandling::Parsed(self.build_tool_call_pipeline(&tools)?)
        } else {
            ToolCallHandling::Streamed
        };
        let bare_json_tool_calls = if tools.is_empty() {
            BareJsonToolCalls::Ignore
        } else {
            BareJsonToolCalls::Detect
        };

        Ok(PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            max_tokens,
            prompt,
            slot_guard,
            token_classification: TokenClassification {
                bare_json_tool_calls,
                streaming_markers: streaming_markers.clone(),
            },
            token_sampling,
            tool_call_handling,
        })
    }

    fn generation_command<TParams>(
        &self,
        agent_name: Option<&str>,
        request: AgentRequest<TParams, GeneratedTokenResult>,
        prepare: fn(
            &Self,
            AgentRequest<TParams, GeneratedTokenResult>,
        ) -> Result<PreparedGenerationRequest, TextGenerationError>,
    ) -> Option<PreparedGenerationRequest> {
        let generated_tokens_tx = request.response_tx.clone();

        match prepare(self, request) {
            Ok(prepared) => Some(prepared),
            Err(rejection) => {
                rejection.report(agent_name, &generated_tokens_tx);

                None
            }
        }
    }

    fn decode_image_bitmap(&self, image_url: &ImageUrl) -> Result<MtmdBitmap, TextGenerationError> {
        DecodedImage::from_data_uri(image_url, self.image_resize_to_fit)
            .map_err(TextGenerationError::ImageDecodingFailed)?
            .to_mtmd_bitmap()
            .map_err(TextGenerationError::ImageBitmapCreationFailed)
    }

    fn build_token_sampling(
        &self,
        grammar: Option<GrammarConstraint>,
    ) -> Result<TokenSampling, TextGenerationError> {
        let grammar = resolve_grammar(grammar, &self.loaded_llama_model.model)?;
        let chain = self
            .sampler_chain_factory
            .create(rng().random::<u32>())
            .map_err(TextGenerationError::SamplerChainCreationFailed)?;

        Ok(TokenSampling { chain, grammar })
    }

    fn build_tool_call_pipeline(
        &self,
        tools: &[Tool<ValidatedParametersSchema>],
    ) -> Result<ToolCallPipeline, TextGenerationError> {
        let validator =
            ToolCallValidator::from_tools(tools).map_err(TextGenerationError::ToolSchemaInvalid)?;
        let tools_json = to_string(tools).map_err(TextGenerationError::ToolsSerializationFailed)?;
        let chat_message_parser =
            ChatMessageParser::new(&self.loaded_llama_model.model, &tools_json)
                .map_err(TextGenerationError::ToolCallParserCreationFailed)?;

        Ok(ToolCallPipeline::new(chat_message_parser, validator))
    }
}

impl PreparesSchedulerCommand for GenerationRequestPreparer {
    type Command = PreparedGenerationRequest;
    type Request = TextGenerationRequest;

    fn prepare_scheduler_command(
        &self,
        agent_name: Option<&str>,
        request: Self::Request,
    ) -> Option<Self::Command> {
        match request {
            TextGenerationRequest::ContinueFromConversationHistory(request) => {
                self.generation_command(agent_name, request, Self::prepare_conversation_history)
            }
            TextGenerationRequest::ContinueFromRawPrompt(request) => {
                self.generation_command(agent_name, request, Self::prepare_raw_prompt)
            }
        }
    }
}
